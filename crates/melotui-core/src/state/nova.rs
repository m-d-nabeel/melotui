//! Nova: particles sprayed out of a pulsing core by the music.
//!
//! Round the core, each direction stands for a stretch of the spectrum, as
//! Nova's colours always ran: the bass straight down, the treble straight
//! up, both sides alike. Every stretch sprays particles its way as long as
//! it sounds, more and farther the louder it is, so the spray takes the
//! music's shape. A kick — the kick-drum range rising suddenly over where it
//! has been — throws a ring out all round besides, swells the core and sends
//! a shockwave out. Every particle slows with drag, sags a little under
//! gravity and fades as it goes. Positions are in units of the distance from
//! the core to the panel's edge, the core at the origin, up positive.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// The bands of the analyser's 512 (30 Hz to 18 kHz, evenly in pitch) that
/// hold kick drums, about 50 to 120 Hz.
const KICK_BANDS: std::ops::Range<usize> = 41..111;

/// A kick is the kick range rising this far over its recent level, at least
/// `MIN_KICK` in the analyser's units, no sooner than `KICK_GAP` after the
/// last: one drum hit, one kick. The analyser's heights go as amplitude to
/// the 0.35, so this is the amplitude rising by two thirds, and `KICK_FULL`
/// it quadrupling, the hardest kick.
const KICK_RISE: f32 = 1.2;
const KICK_FULL: f32 = 1.62;
const MIN_KICK: f32 = 0.05;
const KICK_GAP: f32 = 0.15;
/// How long the recent level takes to follow the music, in seconds.
const RECENT_SECS: f32 = 0.5;

/// How many stretches of the spectrum each side of the core sprays.
const STRETCHES: usize = 48;
/// A stretch this loud, in the analyser's units, sprays `SPRAY` particles a
/// second; quieter ones fewer, as the square of how loud they are.
const LOUD: f32 = 0.3;
const SPRAY: f32 = 20.0;

const DRAG: f32 = 2.2;
const GRAVITY: f32 = 0.3;
const MAX_PARTICLES: usize = 900;

/// Where particles start: the core's edge, in units of the panel's reach.
pub const CORE: f32 = 0.12;

/// The mean of `bands` over `range`, as far as they go.
pub(crate) fn mean(bands: &[f32], range: std::ops::Range<usize>) -> f32 {
    let range = range.start.min(bands.len())..range.end.min(bands.len());
    let n = range.len().max(1) as f32;
    bands[range].iter().sum::<f32>() / n
}

/// How loud the kick-drum range is in the analyser's `bands`: what a
/// visualizer pulses with. The lowest band alone is sub-bass rumble that many
/// songs hardly have.
pub fn kick_level(bands: &[f32]) -> f32 {
    mean(bands, KICK_BANDS)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub life: f32,
    /// Where on the palette it is drawn, 0 for the bass to 1 for the treble.
    pub hue: f32,
}

impl Particle {
    /// How much of its life it has left, 1 when new to 0 when gone.
    pub fn freshness(&self) -> f32 {
        (1.0 - self.age / self.life).clamp(0.0, 1.0)
    }
}

/// Finds kicks in the kick range's level, frame by frame.
#[derive(Debug, Default)]
pub struct KickDetector {
    recent: Option<f32>,
    wait: f32,
}

impl KickDetector {
    /// The strength, 0 to 1, of a kick starting with `level` after `dt`
    /// seconds, if one does.
    pub fn feed(&mut self, level: f32, dt: f32) -> Option<f32> {
        let recent = *self.recent.get_or_insert(level);
        self.wait -= dt;
        let rise = level / recent.max(1e-4);
        let kick = (self.wait <= 0.0 && level >= MIN_KICK && rise >= KICK_RISE)
            .then(|| ((rise - KICK_RISE) / (KICK_FULL - KICK_RISE)).clamp(0.0, 1.0));
        if kick.is_some() {
            self.wait = KICK_GAP;
        }
        self.recent = Some(recent + (level - recent) * (1.0 - (-dt / RECENT_SECS).exp()));
        kick
    }
}

#[derive(Debug)]
pub struct Nova {
    pub particles: Vec<Particle>,
    /// How swollen the core is, 0 to 1: it swells on a kick and settles.
    pub pulse: f32,
    /// Seconds since the last kick, and how hard it was, for its shockwave.
    pub since_kick: f32,
    pub kick_strength: f32,
    kicks: KickDetector,
    /// Particles each stretch owes but has not let out yet.
    owed: [f32; STRETCHES],
    seed: u32,
}

impl Default for Nova {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            pulse: 0.0,
            since_kick: f32::INFINITY,
            kick_strength: 0.0,
            kicks: KickDetector::default(),
            owed: [0.0; STRETCHES],
            seed: 0x9E37_79B9,
        }
    }
}

impl Nova {
    /// Moves everything on by `dt` seconds, with `bands` the analyser's
    /// bands now, empty or all zero when the music has stopped. Returns
    /// whether anything is still to be seen.
    pub fn step(&mut self, bands: &[f32], dt: f32) -> bool {
        self.since_kick += dt;
        self.pulse = (self.pulse - 2.5 * dt).max(0.0);
        if let Some(strength) = self.kicks.feed(kick_level(bands), dt) {
            self.burst(strength);
        }
        self.spray(bands, dt);

        let drag = (-DRAG * dt).exp();
        for p in &mut self.particles {
            p.vx *= drag;
            p.vy = p.vy * drag - GRAVITY * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.age += dt;
        }
        self.particles
            .retain(|p| p.age < p.life && p.x.abs() < 1.4 && p.y.abs() < 1.4);
        if self.particles.len() > MAX_PARTICLES {
            let extra = self.particles.len() - MAX_PARTICLES;
            self.particles.drain(..extra);
        }
        !self.particles.is_empty() || self.pulse > 0.0
    }

    /// Each stretch of the spectrum sprays its way, as loud as it is.
    fn spray(&mut self, bands: &[f32], dt: f32) {
        if bands.is_empty() {
            return;
        }
        for stretch in 0..STRETCHES {
            let start = stretch * bands.len() / STRETCHES;
            let end = ((stretch + 1) * bands.len() / STRETCHES).max(start + 1);
            let loudness = (mean(bands, start..end) / LOUD).min(1.5);
            self.owed[stretch] += SPRAY * loudness * loudness * dt;
            while self.owed[stretch] >= 1.0 {
                self.owed[stretch] -= 1.0;
                let hue = (stretch as f32 + self.random()) / STRETCHES as f32;
                // From straight down for the bass to straight up for the
                // treble, on the right; the left mirrors it.
                let right = -FRAC_PI_2 + hue * PI;
                let angle = if self.random() < 0.5 {
                    right
                } else {
                    PI - right
                };
                let speed = (0.5 + 1.8 * loudness) * (0.85 + 0.3 * self.random());
                let life = 0.5 + 0.4 * self.random();
                self.launch(angle, speed, life, hue);
            }
        }
    }

    /// A ring of particles all round the core, for a kick of `strength`.
    fn burst(&mut self, strength: f32) {
        self.since_kick = 0.0;
        self.kick_strength = strength;
        self.pulse = (0.4 + 0.6 * strength).max(self.pulse);
        let count = (16.0 + 32.0 * strength) as usize;
        let turn = self.random();
        for i in 0..count {
            let angle = (i as f32 + self.random() * 0.6) / count as f32 * TAU + turn;
            let speed = (0.9 + strength) * (0.8 + 0.4 * self.random());
            let life = 0.8 + 0.5 * self.random();
            // Coloured as the stretch of spectrum that way.
            let hue = 0.5 + 0.5 * angle.sin();
            self.launch(angle, speed, life, hue);
        }
    }

    fn launch(&mut self, angle: f32, speed: f32, life: f32, hue: f32) {
        let (sin, cos) = angle.sin_cos();
        self.particles.push(Particle {
            x: cos * CORE,
            y: sin * CORE,
            vx: cos * speed,
            vy: sin * speed,
            age: 0.0,
            life,
            hue,
        });
    }

    /// A number from 0 to 1, from a small generator of its own, so a run is
    /// the same each time it is tested.
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bands(kick: f32) -> Vec<f32> {
        let mut b = vec![0.0; 512];
        b[KICK_BANDS].fill(kick);
        b
    }

    #[test]
    fn a_kick_is_a_sudden_rise_and_fires_once() {
        let mut kicks = KickDetector::default();
        for _ in 0..120 {
            assert_eq!(
                kicks.feed(0.1, 1.0 / 60.0),
                None,
                "a steady level is no kick"
            );
        }
        let strength = kicks.feed(0.25, 1.0 / 60.0).expect("a kick");
        assert!(strength > 0.5, "{strength}");
        assert_eq!(kicks.feed(0.26, 1.0 / 60.0), None, "one drum hit, one kick");
        // Quiet noise rising is not a kick.
        let mut kicks = KickDetector::default();
        kicks.feed(0.001, 1.0 / 60.0);
        assert_eq!(kicks.feed(0.01, 1.0 / 60.0), None);
    }

    #[test]
    fn the_spray_points_where_the_music_is() {
        // Only the treble sounding: particles go upward.
        let mut treble = vec![0.0; 512];
        treble[400..512].fill(0.3);
        let mut nova = Nova::default();
        for _ in 0..30 {
            nova.step(&treble, 1.0 / 60.0);
        }
        assert!(nova.particles.len() > 20, "{}", nova.particles.len());
        assert!(nova.particles.iter().all(|p| p.hue > 0.7));
        let up = nova.particles.iter().filter(|p| p.y > 0.0).count();
        assert!(
            up * 10 >= nova.particles.len() * 9,
            "{up} of {}",
            nova.particles.len()
        );

        // Only the bass: downward.
        let mut bass = vec![0.0; 512];
        bass[0..60].fill(0.3);
        let mut nova = Nova::default();
        for _ in 0..30 {
            nova.step(&bass, 1.0 / 60.0);
        }
        let down = nova.particles.iter().filter(|p| p.y < 0.0).count();
        assert!(down * 10 >= nova.particles.len() * 9);
    }

    #[test]
    fn louder_sprays_more_and_farther() {
        let run = |level: f32| {
            let mut nova = Nova::default();
            let mut mid = vec![0.0; 512];
            mid[200..300].fill(level);
            for _ in 0..60 {
                nova.step(&mid, 1.0 / 60.0);
            }
            let reach = nova
                .particles
                .iter()
                .map(|p| p.x.hypot(p.y))
                .fold(0.0, f32::max);
            (nova.particles.len(), reach)
        };
        let (quiet, quiet_reach) = run(0.1);
        let (loud, loud_reach) = run(0.3);
        assert!(loud > quiet * 4, "{loud} against {quiet}");
        assert!(
            loud_reach > quiet_reach,
            "{loud_reach} against {quiet_reach}"
        );
    }

    #[test]
    fn a_kick_throws_a_ring_out_that_slows_and_fades() {
        let mut nova = Nova::default();
        for _ in 0..60 {
            nova.step(&bands(0.1), 1.0 / 60.0);
        }
        let before = nova.particles.len();
        nova.step(&bands(0.3), 1.0 / 60.0);
        assert!(
            nova.particles.len() >= before + 30,
            "{before} -> {}",
            nova.particles.len()
        );
        assert!(nova.pulse > 0.8 && nova.since_kick < 0.02);
        for _ in 0..120 {
            nova.step(&[], 1.0 / 60.0);
        }
        assert!(nova.particles.is_empty(), "gone within their life");
    }

    #[test]
    fn with_the_music_stopped_it_comes_to_rest() {
        let mut nova = Nova::default();
        let loud = vec![0.25; 512];
        for _ in 0..30 {
            nova.step(&loud, 1.0 / 60.0);
        }
        let mut frames = 0;
        while nova.step(&[], 1.0 / 60.0) {
            frames += 1;
            assert!(frames < 200, "never came to rest");
        }
        assert!(nova.particles.is_empty());
    }
}
