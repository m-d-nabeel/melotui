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

pub fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if !state.should_draw_spectrum() {
        return;
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

    if *state.spectrum_style() == SpectrumStyle::Cava {
        state.spectrum_smooth.copy_from_slice(&state.spectrum_raw);
        for i in 0..len {
            if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
                state.spectrum_peaks[i] = state.spectrum_smooth[i];
            } else {
                state.spectrum_peaks[i] -= 2.0 * dt;
                if state.spectrum_peaks[i] < 0.0 {
                    state.spectrum_peaks[i] = 0.0;
                }
            }
        }
        let take_n = len.clamp(1, 64);
        let energy: f32 = state.spectrum_smooth.iter().take(take_n).sum::<f32>() / take_n as f32;
        let speed = 0.4 + (energy * 18.0).min(5.0);
        state.swirl_phase += speed * dt;
        return;
    }

    let gravity = 4.0;
    let smoothing_speed = 15.0;

    for i in 0..len {
        let target = state.spectrum_raw[i];
        let diff = target - state.spectrum_smooth[i];
        state.spectrum_smooth[i] += diff * (smoothing_speed * dt).min(1.0);

        if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
            state.spectrum_peaks[i] = state.spectrum_smooth[i];
        } else {
            state.spectrum_peaks[i] -= gravity * dt;
            if state.spectrum_peaks[i] < 0.0 {
                state.spectrum_peaks[i] = 0.0;
            }
        }
    }

    let take_n = len.clamp(1, 64);
    let energy: f32 = state.spectrum_smooth.iter().take(take_n).sum::<f32>() / take_n as f32;
    let speed = 0.4 + (energy * 18.0).min(5.0);
    state.swirl_phase += speed * dt;
}
