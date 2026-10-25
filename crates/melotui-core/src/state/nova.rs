//! Nova: particles thrown out of a pulsing core by the music's kicks.
//!
//! A kick is a sudden rise in the kick-drum range well above where it has
//! been lately; it throws a ring of particles out of the core, more of them
//! and faster the harder it is. Between kicks, the hi-hats and voices let a
//! trickle of slower ones out. Every particle slows with drag, sags a little
//! under gravity and fades as it goes. Positions are in units of the
//! distance from the core to the panel's edge, the core at the origin, up
//! positive.

use std::f32::consts::TAU;

/// The bands of the analyser's 512 (30 Hz to 18 kHz, evenly in pitch) that
/// hold kick drums, about 50 to 120 Hz, and hi-hats and voices, about 1 to
/// 8 kHz.
const KICK_BANDS: std::ops::Range<usize> = 41..111;
const BRIGHT_BANDS: std::ops::Range<usize> = 250..420;

/// A kick is the kick range rising this far over its recent level, at least
/// `MIN_KICK` in the analyser's units, no sooner than `KICK_GAP` after the
/// last: one drum hit, one kick.
const KICK_RISE: f32 = 1.3;
const MIN_KICK: f32 = 0.05;
const KICK_GAP: f32 = 0.15;
/// How long the recent level takes to follow the music, in seconds.
const RECENT_SECS: f32 = 0.5;

const DRAG: f32 = 2.2;
const GRAVITY: f32 = 0.35;
const MAX_PARTICLES: usize = 700;

/// The mean of `bands` over `range`, as far as they go.
fn mean(bands: &[f32], range: std::ops::Range<usize>) -> f32 {
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
    /// Where on the palette it is drawn, 0 to 1.
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
            .then(|| ((rise - KICK_RISE) / (2.0 - KICK_RISE)).clamp(0.0, 1.0));
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
    /// Trickle particles owed but not yet let out.
    owed: f32,
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
            owed: 0.0,
            seed: 0x9E37_79B9,
        }
    }
}

impl Nova {
    /// Moves everything on by `dt` seconds, with `bands` the analyser's
    /// bands now, all zero when the music has stopped. Returns whether
    /// anything is still to be seen.
    pub fn step(&mut self, bands: &[f32], dt: f32) -> bool {
        let (kick, bright) = (kick_level(bands), mean(bands, BRIGHT_BANDS));

        self.since_kick += dt;
        self.pulse = (self.pulse - 2.5 * dt).max(0.0);
        if let Some(strength) = self.kicks.feed(kick, dt) {
            self.burst(strength);
        }
        self.owed += bright * 120.0 * dt;
        while self.owed >= 1.0 {
            self.owed -= 1.0;
            self.trickle();
        }

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

    /// A ring of particles out of the core, for a kick of `strength`.
    fn burst(&mut self, strength: f32) {
        self.since_kick = 0.0;
        self.kick_strength = strength;
        self.pulse = (0.4 + 0.6 * strength).max(self.pulse);
        let count = (24.0 + 48.0 * strength) as usize;
        let turn = self.random();
        for i in 0..count {
            let angle = (i as f32 + self.random() * 0.6) / count as f32 * TAU + turn;
            let speed = (0.8 + strength) * (0.8 + 0.4 * self.random());
            let life = 0.9 + 0.6 * self.random();
            self.launch(angle, speed, life, CORE);
        }
    }

    /// One slow particle, for the brighter sounds between kicks.
    fn trickle(&mut self) {
        let angle = self.random() * TAU;
        let speed = 0.25 + 0.3 * self.random();
        let life = 0.5 + 0.4 * self.random();
        self.launch(angle, speed, life, CORE);
    }

    fn launch(&mut self, angle: f32, speed: f32, life: f32, from: f32) {
        let (sin, cos) = angle.sin_cos();
        // Bass at the bottom of the palette's turn, treble at the top, as
        // Nova's colours have always run.
        let hue = 0.5 - 0.5 * sin;
        self.particles.push(Particle {
            x: cos * from,
            y: sin * from,
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

/// Where particles start: the core's edge, in units of the panel's reach.
pub const CORE: f32 = 0.12;

#[cfg(test)]
mod tests {
    use super::*;

    fn bands(kick: f32, bright: f32) -> Vec<f32> {
        let mut b = vec![0.0; 512];
        b[KICK_BANDS].fill(kick);
        b[BRIGHT_BANDS].fill(bright);
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
    fn a_kick_throws_particles_out_that_slow_and_fade() {
        let mut nova = Nova::default();
        for _ in 0..60 {
            nova.step(&bands(0.1, 0.0), 1.0 / 60.0);
        }
        assert!(nova.particles.is_empty());
        nova.step(&bands(0.3, 0.0), 1.0 / 60.0);
        let thrown = nova.particles.len();
        assert!(thrown >= 40, "{thrown} particles");
        assert!(nova.pulse > 0.8 && nova.since_kick < 0.02);

        let speed = |n: &Nova| {
            n.particles.iter().map(|p| p.vx.hypot(p.vy)).sum::<f32>() / n.particles.len() as f32
        };
        let reach = |n: &Nova| {
            n.particles.iter().map(|p| p.x.hypot(p.y)).sum::<f32>() / n.particles.len() as f32
        };
        let (s0, r0) = (speed(&nova), reach(&nova));
        for _ in 0..20 {
            nova.step(&bands(0.3, 0.0), 1.0 / 60.0);
        }
        assert!(speed(&nova) < s0 * 0.6, "drag slows them");
        assert!(reach(&nova) > r0 + 0.2, "they fly outward");
        for _ in 0..120 {
            nova.step(&bands(0.0, 0.0), 1.0 / 60.0);
        }
        assert!(nova.particles.is_empty(), "and are gone within their life");
    }

    #[test]
    fn brighter_sounds_let_a_trickle_out_between_kicks() {
        let mut nova = Nova::default();
        for _ in 0..60 {
            nova.step(&bands(0.0, 0.2), 1.0 / 60.0);
        }
        // 0.2 * 120 particles a second, less those already faded.
        assert!(
            (10..=30).contains(&nova.particles.len()),
            "{}",
            nova.particles.len()
        );
    }

    #[test]
    fn with_the_music_stopped_it_comes_to_rest() {
        let mut nova = Nova::default();
        nova.step(&bands(0.1, 0.2), 1.0 / 60.0);
        nova.step(&bands(0.4, 0.2), 1.0 / 60.0);
        let mut frames = 0;
        while nova.step(&[0.0; 512], 1.0 / 60.0) {
            frames += 1;
            assert!(frames < 200, "never came to rest");
        }
        assert!(nova.particles.is_empty());
    }
}
