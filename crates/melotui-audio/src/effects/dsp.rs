//! Small building blocks shared by the hand-written units in this module.

use fundsp::svf::SvfCoefs;

/// Below this, a recursive filter's state is rounded to zero; see
/// [`flush_denormal`].
const DENORMAL_FLOOR: f32 = 1e-20;

/// How long a [`Glide`] takes to cover 90% of a change. Quick enough to feel
/// immediate, slow enough that the change is a glide rather than a step.
pub const GLIDE_SECONDS: f32 = 0.03;

/// Highest frequency any filter here may be tuned to, as a fraction of the
/// sample rate.
///
/// The filters are prewarped with `tan(π·f/fs)`, which is infinite at Nyquist
/// and negative beyond it, and a filter tuned there does not misbehave
/// politely: its state fills with infinities, and a single NaN anywhere in the
/// graph silences everything downstream. A 16 kHz EQ band is exactly at
/// Nyquist for a 32 kHz file, and the 20 kHz low-pass is above it for anything
/// under 40 kHz.
pub const NYQUIST_GUARD: f32 = 0.45;

/// Rounds a recursive filter's state to zero once it is far below audibility.
///
/// A recursive filter fed digital silence — a pause, the gap between two
/// tracks — decays geometrically toward zero and, without this, spends that
/// time in subnormal floats, which the CPU handles in microcode: four times
/// the cost of the same filters fed music, on the thread where a stall is a
/// dropout.
pub fn flush_denormal(x: f32) -> f32 {
    if x.abs() < DENORMAL_FLOOR { 0.0 } else { x }
}

/// A value that follows its target through three one-pole stages in series,
/// so a change starts and ends without a corner — the same shape as fundsp's
/// `follow`.
///
/// Once every stage is within a hair of the target it snaps exactly onto it
/// and reports itself settled, which is what lets the units built on it skip
/// recomputing coefficients, and skip filters entirely, whenever nothing is
/// moving.
///
/// The stages are kept in `f64`. In `f32` a one-pole this slow stops short of
/// its target for good: once `coeff × remaining` is under half an ulp of the
/// value, the update rounds to nothing, which at 48 kHz left a glide stuck
/// with a few hundred-thousandths of the change still to go — and four times
/// further out at 192 kHz.
#[derive(Clone, Copy)]
pub struct Glide {
    stages: [f64; 3],
    settled: bool,
}

impl Glide {
    pub fn new(value: f32) -> Self {
        Self {
            stages: [value as f64; 3],
            settled: true,
        }
    }

    pub fn value(&self) -> f32 {
        self.stages[2] as f32
    }

    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// Jumps straight to `value`, for a fresh stream with nothing to be
    /// continuous with.
    pub fn snap(&mut self, value: f32) {
        *self = Self::new(value);
    }

    /// Moves one sample toward `target`, and reports whether the value moved.
    pub fn step(&mut self, target: f32, coeff: f32) -> bool {
        let (target, coeff) = (target as f64, coeff as f64);
        if self.settled && self.stages[2] == target {
            return false;
        }
        let [a, b, c] = &mut self.stages;
        *a += coeff * (target - *a);
        *b += coeff * (*a - *b);
        *c += coeff * (*b - *c);
        let tolerance = 1e-6 * target.abs().max(1.0);
        self.settled = self.stages.iter().all(|s| (s - target).abs() <= tolerance);
        if self.settled {
            self.stages = [target; 3];
        }
        true
    }
}

/// The per-stage coefficient that makes a [`Glide`] take [`GLIDE_SECONDS`].
pub fn glide_coeff(sample_rate: f32) -> f32 {
    // Three equal one-poles in series cover 90% of a step in about 5.32 of
    // their time constants.
    let time_constant = GLIDE_SECONDS / 5.32;
    1.0 - (-1.0 / (time_constant * sample_rate.max(1.0))).exp()
}

/// The state of one trapezoidal state-variable filter, run with fundsp's
/// coefficients so that every response here is the one fundsp's own filter
/// nodes would give.
#[derive(Clone, Copy, Default)]
pub struct SvfState {
    ic1: f32,
    ic2: f32,
}

impl SvfState {
    pub fn tick(&mut self, coefs: &SvfCoefs<f32>, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = coefs.a1 * self.ic1 + coefs.a2 * v3;
        let v2 = self.ic2 + coefs.a2 * self.ic1 + coefs.a3 * v3;
        self.ic1 = flush_denormal(2.0 * v1 - self.ic1);
        self.ic2 = flush_denormal(2.0 * v2 - self.ic2);
        coefs.m0 * x + coefs.m1 * v1 + coefs.m2 * v2
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glide_arrives_exactly_and_then_stops_moving() {
        let coeff = glide_coeff(48_000.0);
        let mut g = Glide::new(0.0);
        let mut moved_at = 0;
        for i in 0..48_000 {
            if g.step(1.0, coeff) {
                moved_at = i;
            }
        }
        assert_eq!(g.value(), 1.0, "settled exactly on the target");
        assert!(g.is_settled());
        assert!(
            moved_at < 9_600,
            "still creeping after {moved_at} samples; a glide should be over in a fraction of a second"
        );
    }

    #[test]
    fn a_glide_covers_most_of_a_change_in_its_glide_time() {
        let sample_rate = 48_000.0;
        let coeff = glide_coeff(sample_rate);
        let mut g = Glide::new(0.0);
        let n = (GLIDE_SECONDS * sample_rate) as usize;
        for _ in 0..n {
            g.step(1.0, coeff);
        }
        assert!(
            (0.85..0.95).contains(&g.value()),
            "covered {:.3} of the change in the glide time",
            g.value()
        );
    }

    #[test]
    fn a_glide_starts_without_a_corner() {
        // The reason for three stages rather than one: a single one-pole sets
        // off at its fastest, so the value bends sharply where the change
        // lands. Here the first steps are tiny and grow.
        let coeff = glide_coeff(48_000.0);
        let mut g = Glide::new(0.0);
        g.step(1.0, coeff);
        let first = g.value();
        g.step(1.0, coeff);
        let second = g.value() - first;
        assert!(
            first < 1e-6 && second > first,
            "a glide should ease in: first step {first:e}, second {second:e}"
        );
    }
}
