//! The Particle style: droplets thrown up from a line by the music.
//!
//! Across the panel, each column stands for a stretch of the spectrum, the
//! bass in the middle. Each throws droplets up from the centre line as long
//! as it sounds, more the louder it is and each fast enough to rise about as
//! high as it is loud, and gravity brings them back down; the panel mirrors
//! them below the line and either side. So the spectrum is drawn by where
//! the droplets turn, as a cloud of them rather than as solid bars.
//! Positions: `x` from 0 for the bass to 1 for the treble, `y` up from the
//! line in units of the panel's half height.

use super::nova::mean;

const COLUMNS: usize = 72;
/// A band this loud, in the analyser's units, throws droplets to the top.
const FULL: f32 = 0.45;
/// A column throwing to the top throws this many droplets a second; quieter
/// ones fewer, falling off a little faster than how high they throw.
const RATE: f32 = 120.0;
const RATE_CURVE: f32 = 1.2;
/// In half heights per second squared: a droplet thrown to the top is back
/// in under a second, one thrown a third of the way in under half a second.
const GRAVITY: f32 = 12.0;
const MAX_DROPS: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    /// How high it was thrown to, 0 to 1, for its colour.
    pub peak: f32,
}

#[derive(Debug)]
pub struct Fountain {
    pub drops: Vec<Drop>,
    /// Droplets each column owes but has not thrown yet.
    owed: [f32; COLUMNS],
    seed: u32,
}

impl Default for Fountain {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            owed: [0.0; COLUMNS],
            seed: 0x2545_F491,
        }
    }
}

impl Fountain {
    /// Moves the droplets on by `dt` seconds, with `bands` the analyser's
    /// bands now, empty or all zero when the music has stopped. Returns
    /// whether any are still in the air.
    pub fn step(&mut self, bands: &[f32], dt: f32) -> bool {
        for d in &mut self.drops {
            d.vy -= GRAVITY * dt;
            d.y += d.vy * dt;
        }
        self.drops.retain(|d| d.y >= 0.0);

        if !bands.is_empty() {
            for column in 0..COLUMNS {
                let start = column * bands.len() / COLUMNS;
                let end = ((column + 1) * bands.len() / COLUMNS).max(start + 1);
                let height = (mean(bands, start..end) / FULL).min(1.0);
                self.owed[column] += RATE * height.powf(RATE_CURVE) * dt;
                while self.owed[column] >= 1.0 {
                    self.owed[column] -= 1.0;
                    let peak = (height * (0.85 + 0.3 * self.random())).min(1.0);
                    let x = (column as f32 + self.random()) / COLUMNS as f32;
                    self.drops.push(Drop {
                        x,
                        y: 0.0,
                        vy: (2.0 * GRAVITY * peak).sqrt(),
                        peak,
                    });
                }
            }
        }
        if self.drops.len() > MAX_DROPS {
            let extra = self.drops.len() - MAX_DROPS;
            self.drops.drain(..extra);
        }
        !self.drops.is_empty()
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

    #[test]
    fn droplets_rise_about_as_high_as_their_band_is_loud() {
        let mut bands = vec![0.0; 512];
        bands[..64].fill(FULL * 0.5);
        let mut fountain = Fountain::default();
        let mut highest = 0.0f32;
        for _ in 0..120 {
            fountain.step(&bands, 1.0 / 60.0);
            highest = fountain.drops.iter().map(|d| d.y).fold(highest, f32::max);
        }
        assert!((0.42..0.6).contains(&highest), "rose to {highest}");
        assert!(
            fountain
                .drops
                .iter()
                .all(|d| d.x < 64.0 / 512.0 + 1.0 / 72.0),
            "only the bass columns throw"
        );
    }

    #[test]
    fn louder_throws_more() {
        let count = |level: f32| {
            let mut fountain = Fountain::default();
            let bands = vec![level; 512];
            for _ in 0..60 {
                fountain.step(&bands, 1.0 / 60.0);
            }
            fountain.drops.len()
        };
        assert!(count(0.4) > count(0.1) * 3);
    }

    #[test]
    fn with_the_music_stopped_the_droplets_come_down() {
        let mut fountain = Fountain::default();
        let bands = vec![0.4; 512];
        for _ in 0..60 {
            fountain.step(&bands, 1.0 / 60.0);
        }
        let mut frames = 0;
        while fountain.step(&[], 1.0 / 60.0) {
            frames += 1;
            assert!(frames < 120, "still in the air");
        }
        assert!(frames > 10, "fell rather than vanished: {frames} frames");
    }
}
