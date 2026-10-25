use std::{
    process::{Command, Stdio},
    str::FromStr,
    sync::OnceLock,
};

use crate::AppState;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VisualizerMode {
    Spectrum,
    #[default]
    AlbumArt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectrumStyle {
    Cava,
    Default,
    Bar,
    Particle,
    Radial,
    Nova,
}

/// Whether the external `cava` binary is on `PATH`.
///
/// Probed once and cached. This used to shell out on every call, and callers
/// include `SpectrumStyle::next()` — which runs inside the reducer on the UI
/// thread — so cycling visualizer styles forked a process per keypress.
pub fn is_cava_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("cava")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .status()
            .is_ok()
    })
}

impl Default for SpectrumStyle {
    fn default() -> Self {
        if is_cava_available() {
            tracing::info!("Cava detected, using for spectrum");
            SpectrumStyle::Cava
        } else {
            tracing::info!("Cava not found, using default visualizer");
            tracing::info!("Install CAVA for better spectrum visualization");
            SpectrumStyle::Default
        }
    }
}

impl SpectrumStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpectrumStyle::Cava => "Cava",
            SpectrumStyle::Default => "Default",
            SpectrumStyle::Bar => "Bar",
            SpectrumStyle::Particle => "Particle",
            SpectrumStyle::Radial => "Radial",
            SpectrumStyle::Nova => "Nova",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SpectrumStyle::Cava => SpectrumStyle::Default,
            SpectrumStyle::Default => SpectrumStyle::Bar,
            SpectrumStyle::Bar => SpectrumStyle::Particle,
            SpectrumStyle::Particle => SpectrumStyle::Radial,
            SpectrumStyle::Radial => SpectrumStyle::Nova,
            SpectrumStyle::Nova => {
                if is_cava_available() {
                    SpectrumStyle::Cava
                } else {
                    SpectrumStyle::Default
                }
            }
        }
    }
}

impl FromStr for SpectrumStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cava" => Ok(SpectrumStyle::Cava),
            "default" | "def" => Ok(SpectrumStyle::Default),
            "bar" => Ok(SpectrumStyle::Bar),
            "particle" | "part" | "p" => Ok(SpectrumStyle::Particle),
            "radial" | "rad" | "orbit" => Ok(SpectrumStyle::Radial),
            "nova" | "supernova" | "pulse" | "corona" => Ok(SpectrumStyle::Nova),
            _ => Err(()),
        }
    }
}

/// How each cap is moving, kept from frame to frame.
#[derive(Debug, Default)]
pub struct SpectrumMotion {
    /// How fast each bar is falling, in spectrum units per second.
    bar_speed: Vec<f32>,
    /// How long each cap has left to hold before it falls, in seconds.
    cap_hold: Vec<f32>,
    /// How fast each cap is falling, in spectrum units per second.
    cap_speed: Vec<f32>,
}

/// Bars rise this share of the way to the music each 60th of a second —
/// quickly, but not in one frame, which would flash every loud instant —
/// worked out for any frame time so they move alike at any frame rate.
const BAR_RISE_AT_60_FPS: f32 = 0.6;

/// Bars fall like dropped things, ever faster, in spectrum units per second
/// squared: a typical tall bar of the analyser's lands in about a tenth of a
/// second. Against easing down a quarter of the way each frame, as they did,
/// this moves the bars about twice as much over a song at the same average
/// height: livelier, no denser. cava's bands run to 1 rather than about 0.45,
/// and fall with gravity scaled to match when the music stops.
const BAR_GRAVITY: f32 = 40.0;
const BAR_GRAVITY_CAVA: f32 = 90.0;

/// Caps hold this long at their highest, then fall ever faster, a little
/// slower than their bars so they trail them down rather than hang above
/// them. Over real songs this keeps them as close to their bars as they were
/// when the bars eased down: floating clear of them about a quarter of the
/// time. Gravity is in spectrum units per second squared, scaled up for
/// cava's bands as for its bars.
const CAP_HOLD_SECS: f32 = 0.08;
const CAP_GRAVITY: f32 = 30.0;
const CAP_GRAVITY_CAVA: f32 = 68.0;

/// Moves the bars and caps on by `dt` seconds. Returns whether anything is
/// still moving or showing, so frames are worth drawing: once the music has
/// stopped and everything has come to rest, nothing changes until it plays.
pub fn update_visualizer_physics(state: &mut AppState, dt: f32) -> bool {
    if !state.should_draw_spectrum() {
        return false;
    }

    if state.spectrum_raw.len() < 512 {
        state.spectrum_raw.resize(512, 0.0);
    }

    let count = state.spectrum_source.get_latest(&mut state.spectrum_raw);

    if count > 0 {
        state.spectrum_raw.truncate(count);
    }

    let len = state.spectrum_raw.len();
    if state.spectrum_smooth.len() != len {
        state.spectrum_smooth.resize(len, 0.0);
    }
    if state.spectrum_peaks.len() != len {
        state.spectrum_peaks.resize(len, 0.0);
    }

    let cava = *state.spectrum_style() == SpectrumStyle::Cava;
    // Paused or stopped, nothing new arrives: let everything come to rest
    // instead of standing frozen where the music left it.
    let playing = state.is_playing();
    let rise = 1.0 - (1.0 - BAR_RISE_AT_60_FPS).powf(dt * 60.0);
    let (bar_gravity, cap_gravity) = if cava {
        (BAR_GRAVITY_CAVA, CAP_GRAVITY_CAVA)
    } else {
        (BAR_GRAVITY, CAP_GRAVITY)
    };

    let motion = &mut state.spectrum_motion;
    motion.bar_speed.resize(len, 0.0);
    motion.cap_hold.resize(len, 0.0);
    motion.cap_speed.resize(len, 0.0);
    let mut showing = playing;

    for i in 0..len {
        let target = if playing { state.spectrum_raw[i] } else { 0.0 };
        let bar = &mut state.spectrum_smooth[i];
        if cava && playing {
            // cava smooths its own bars.
            *bar = target;
        } else if target >= *bar {
            *bar += (target - *bar) * rise;
            motion.bar_speed[i] = 0.0;
        } else {
            motion.bar_speed[i] += bar_gravity * dt;
            *bar = (*bar - motion.bar_speed[i] * dt).max(target);
        }

        let cap = &mut state.spectrum_peaks[i];
        if *bar >= *cap {
            *cap = *bar;
            motion.cap_hold[i] = CAP_HOLD_SECS;
            motion.cap_speed[i] = 0.0;
        } else if motion.cap_hold[i] > 0.0 {
            motion.cap_hold[i] -= dt;
        } else {
            motion.cap_speed[i] += cap_gravity * dt;
            *cap = (*cap - motion.cap_speed[i] * dt).max(*bar);
        }
        showing |= *cap > 0.0;
    }

    let take_n = len.clamp(1, 64);
    let energy: f32 = state.spectrum_smooth.iter().take(take_n).sum::<f32>() / take_n as f32;
    let speed = 0.4 + (energy * 18.0).min(5.0);
    state.swirl_phase += speed * dt;
    showing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{playback::state::PlaybackStatus, utils::SpectrumSource};
    use std::sync::{Arc, Mutex};

    /// A source whose single band reads whatever the test puts in it.
    struct Level(Arc<Mutex<f32>>);

    impl SpectrumSource for Level {
        fn get_latest(&self, out: &mut [f32]) -> usize {
            out[0] = *self.0.lock().unwrap();
            1
        }
    }

    fn state(style: SpectrumStyle) -> (AppState, Arc<Mutex<f32>>) {
        let library = crate::library::snapshot::Library::new(
            vec![],
            vec![],
            std::collections::HashMap::new(),
        );
        let mut state = AppState::new(library, &crate::config::AppConfig::default());
        state.session.visualizer_mode = VisualizerMode::Spectrum;
        state.session.spectrum_style = style;
        state.session.playback.status = PlaybackStatus::Playing;
        let level = Arc::new(Mutex::new(0.0));
        state.spectrum_source = Box::new(Level(level.clone()));
        (state, level)
    }

    /// Runs `frames` frames of `dt`, returning the bar and cap after each.
    fn run(state: &mut AppState, frames: usize, dt: f32) -> Vec<(f32, f32)> {
        (0..frames)
            .map(|_| {
                update_visualizer_physics(state, dt);
                (state.spectrum_smooth[0], state.spectrum_peaks[0])
            })
            .collect()
    }

    #[test]
    fn bars_rise_most_of_the_way_at_once_and_fall_ever_faster() {
        let (mut state, level) = state(SpectrumStyle::Default);
        *level.lock().unwrap() = 0.3;
        let up = run(&mut state, 3, 1.0 / 60.0);
        assert!((up[0].0 - 0.18).abs() < 1e-5, "first frame: {}", up[0].0);
        assert!(up[2].0 > 0.28, "three frames in: {}", up[2].0);

        let top = state.spectrum_smooth[0];
        *level.lock().unwrap() = 0.0;
        let bars: Vec<f32> = run(&mut state, 30, 1.0 / 60.0)
            .iter()
            .map(|f| f.0)
            .collect();
        let landed = bars.iter().position(|b| *b == 0.0).unwrap();
        let drops: Vec<f32> = std::iter::once(top)
            .chain(bars[..landed].iter().copied())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| w[0] - w[1])
            .collect();
        assert!(
            drops[..drops.len() - 1].windows(2).all(|d| d[1] > d[0]),
            "the fall speeds up: {drops:?}"
        );
        // sqrt(2 * 0.3 / 40) s is about 7 frames.
        assert!((6..=9).contains(&landed), "landed after {landed} frames");
    }

    #[test]
    fn bars_move_alike_at_any_frame_rate() {
        let fall_time = |fps: f32| {
            let (mut state, level) = state(SpectrumStyle::Default);
            *level.lock().unwrap() = 0.3;
            run(&mut state, (fps * 0.3) as usize, 1.0 / fps);
            *level.lock().unwrap() = 0.0;
            let mut t = 0.0;
            while state.spectrum_smooth[0] > 0.0 {
                update_visualizer_physics(&mut state, 1.0 / fps);
                t += 1.0 / fps;
            }
            t
        };
        let at_60 = fall_time(60.0);
        for fps in [30.0, 144.0] {
            let t = fall_time(fps);
            assert!(
                (t - at_60).abs() < 0.04,
                "{t} s at {fps} fps, {at_60} s at 60"
            );
        }
    }

    #[test]
    fn once_stopped_and_at_rest_there_is_nothing_to_draw() {
        let (mut state, level) = state(SpectrumStyle::Default);
        *level.lock().unwrap() = 0.0;
        assert!(
            update_visualizer_physics(&mut state, 1.0 / 60.0),
            "playing: drawn"
        );
        *level.lock().unwrap() = 0.4;
        run(&mut state, 30, 1.0 / 60.0);
        state.session.playback.status = PlaybackStatus::Paused;
        let mut frames = 0;
        while update_visualizer_physics(&mut state, 1.0 / 60.0) {
            frames += 1;
            assert!(frames < 120, "never came to rest");
        }
        assert!(
            frames > 5,
            "the bars fell rather than vanished: {frames} frames"
        );
        assert!(!update_visualizer_physics(&mut state, 1.0 / 60.0));
    }

    #[test]
    fn a_cap_holds_then_falls_ever_faster_onto_its_bar() {
        let (mut state, level) = state(SpectrumStyle::Default);
        *level.lock().unwrap() = 0.3;
        run(&mut state, 60, 1.0 / 60.0);
        let top = state.spectrum_peaks[0];
        *level.lock().unwrap() = 0.0;
        let frames = run(&mut state, 120, 1.0 / 60.0);
        let hold = (CAP_HOLD_SECS * 60.0) as usize;
        assert!(frames[..hold].iter().all(|f| f.1 == top), "the cap holds");

        let caps: Vec<f32> = frames.iter().map(|f| f.1).collect();
        // A cap comes to rest on its bar.
        let landed = frames.iter().position(|(bar, cap)| cap == bar).unwrap();
        let drops: Vec<f32> = caps[hold..landed].windows(2).map(|w| w[0] - w[1]).collect();
        assert!(
            drops.windows(2).all(|d| d[1] > d[0]),
            "the fall speeds up: {drops:?}"
        );
        let falling = (landed - hold) as f32 / 60.0;
        assert!(
            (0.08..0.25).contains(&falling),
            "a cap from {top} fell for {falling} s"
        );
    }

    #[test]
    fn cava_bars_follow_cava_and_their_caps_fall_alike() {
        let (mut state, level) = state(SpectrumStyle::Cava);
        *level.lock().unwrap() = 0.67;
        let up = run(&mut state, 30, 1.0 / 60.0);
        assert_eq!(up.last().unwrap().0, 0.67, "cava smooths its own bars");
        *level.lock().unwrap() = 0.0;
        let frames = run(&mut state, 120, 1.0 / 60.0);
        assert_eq!(frames[0].0, 0.0);
        let hold = (CAP_HOLD_SECS * 60.0) as usize;
        let landed = frames.iter().position(|f| f.1 == 0.0).unwrap();
        let falling = (landed - hold) as f32 / 60.0;
        assert!((0.08..0.25).contains(&falling), "{falling} s");
    }

    #[test]
    fn paused_everything_comes_to_rest_instead_of_freezing() {
        for style in [SpectrumStyle::Default, SpectrumStyle::Cava] {
            let (mut state, level) = state(style.clone());
            *level.lock().unwrap() = 0.4;
            run(&mut state, 30, 1.0 / 60.0);
            state.session.playback.status = PlaybackStatus::Paused;
            let frames = run(&mut state, 120, 1.0 / 60.0);
            let (bar, cap) = *frames.last().unwrap();
            assert!(bar < 0.001 && cap < 0.001, "{style:?}: {bar}, {cap}");
        }
    }
}
