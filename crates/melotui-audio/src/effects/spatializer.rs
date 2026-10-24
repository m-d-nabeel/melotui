//! Headphone spatialiser: static pan plus an optional orbiting auto-pan.
//!
//! # Why this was rewritten
//!
//! The previous version summed the input to mono (`(left + right) * 0.5`), fed
//! that single signal through one delay line, and crossfaded up to **80%** of it
//! back over the dry signal. Everything that only exists as a difference between
//! the channels — reverb tails, room ambience, wide-panned instruments — was
//! therefore attenuated by up to 14 dB the moment the effect was switched on.
//! Measured with the left channel carrying a 400 Hz tone and the right a 700 Hz
//! tone, channel separation collapsed from 50 dB to 7.5 dB at depth 0.5, and the
//! bleed was **in phase**: mono summing, not widening.
//!
//! # What it does now
//!
//! Each channel keeps its own delay line and its own direct path at unity, so
//! the stereo difference survives intact. The ears are coupled by *crossfeed* —
//! a small, delayed, low-passed amount of each channel fed to the opposite ear,
//! which is how a real head works and how analogue crossfeed circuits are
//! built. Because the crossfeed is low-passed, it only touches the low end,
//! where the ears genuinely cannot resolve direction; everything above the
//! shadow cutoff keeps its full separation. It is a *shuffler*: each ear gives
//! up as much of its own low band as it receives of the other's, so the bottom
//! end moves toward the centre rather than getting louder.
//!
//! Direction is cued the way it physically is: an interaural time difference
//! (the far ear hears it later), an interaural level difference that grows with
//! frequency, head shadowing (the far ear hears it duller), a pinna notch that
//! moves with front/back, and a room.
//!
//! # Why the room matters most
//!
//! The version before this one had correct ITD, correct crossfeed and a
//! constant-power pan, and still sounded like a pan pot rather than a source in
//! space. The reason is well documented and has nothing to do with the
//! direction cues: dry headphone audio localises *inside the head* no matter
//! how accurate its interaural differences are. What pushes an image out of the
//! head is the room — specifically the pattern of early reflections arriving in
//! the first few tens of milliseconds, which is what tells the auditory system
//! "this source is some distance away in a space".
//!
//! So this stage now carries its own early-reflection bed. It is deliberately
//! *not* the `reverb` parameter: that is a global effect the listener dials in
//! for taste, defaults to zero, and sits downstream. The room here is part of
//! the spatialiser, scales with the orbit depth, and exists to externalise.
//!
//! Two properties of the bed are load-bearing:
//!
//! * It is **ipsilateral** — each ear's reflections are built from that ear's
//!   own post-pan signal. A bed that crossfed would add a second, unbudgeted
//!   path for one channel's content to reach the other ear, which is the exact
//!   failure (image collapse) the previous rewrite existed to fix.
//! * Its two tap patterns are **mutually decorrelated** (no shared delay
//!   times), so the bed widens the image rather than filling the middle of it.
//!
//! # Controls glide, and switching off is a fade
//!
//! Every control scales something in the signal path, so a setting that moves
//! in one step — a keypress, a preset, a reset — is a gain step unless it is
//! smoothed, and a gain step is a click. Measured before the controls were
//! smoothed, switching the orbit off produced a discontinuity some 900 times
//! rougher than the music around it. So every control is followed rather than
//! jumped to, switching off is a glide to zero, and the unit only drops into its
//! bit-transparent bypass once everything has arrived there.

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;
use std::f32::consts::{PI, SQRT_2, TAU};

use super::AudioParams;

/// Maximum interaural time difference, i.e. the head's width in seconds.
const MAX_ITD_SECONDS: f32 = 0.000_65;
/// Above this, the far ear is shadowed by the head. Also the crossfeed cutoff:
/// crossfeeding high frequencies is what would collapse the image.
const SHADOW_CUTOFF_HZ: f32 = 700.0;
/// Crossfeed at full depth. Analogue crossfeed circuits sit near -9 dB; this is
/// -11 dB, and only below the shadow cutoff.
const MAX_CROSSFEED: f32 = 0.28;
/// Crossfeed floor once the spatialiser is engaged at all. Crossfeed is a
/// property of having a head, not of how far the source is off-centre, so
/// scaling it all the way down to nothing at small pan angles was wrong; it
/// only ever reached its useful range at extreme settings. Engaging takes the
/// crossfeed's target straight from nothing to this floor; the glide in
/// [`Amounts`] is what makes that a fade rather than a step.
const MIN_CROSSFEED_SCALE: f32 = 0.5;

/// How fast every control follows its target. About 30 ms to cover most of a
/// change: quick enough to feel immediate, slow enough to be a glide rather
/// than a discontinuity.
const CONTROL_SMOOTHING_HZ: f32 = 12.0;
/// Below this, an amount heading for zero counts as having arrived, and the
/// unit may drop into bypass. Low enough that the room has drained as well —
/// its line still holds 50 ms of what it was fed while its level was higher —
/// so cutting the processed path off here is a step over 100 dB down. Getting
/// here from full scale takes about 200 ms.
const SETTLED: f32 = 1e-7;
/// Below this, a recursive filter's state is rounded to zero; see
/// [`flush_denormal`].
const DENORMAL_FLOOR: f32 = 1e-20;

/// Split point between "the head is not in the way" and "the head is in the
/// way". Below roughly this frequency a wavelength is longer than a head and
/// diffracts around it, which is why interaural *level* differences are a
/// high-frequency cue and interaural *time* differences are a low-frequency
/// one.
const ILD_SPLIT_HZ: f32 = 1_000.0;
/// Extra attenuation applied to the far ear's high band at full deflection,
/// on top of the broadband balance law. Real heads produce 15–20 dB of ILD at
/// 8 kHz and almost none at 200 Hz; the balance law alone is flat with
/// frequency, which is precisely what a mixing-desk pan pot does and precisely
/// why it reads as "inside the head" rather than "over there".
const MAX_HF_SHADOW: f32 = 0.90;

/// Centre of the pinna notch with the source in front, and behind. The outer
/// ear imposes a deep, fairly narrow notch in the top octaves whose centre
/// frequency moves with the source's front/back angle; this is the cue the
/// auditory system actually uses to resolve front from back, and it is why the
/// previous cue — a gentle one-pole low-pass blended in at 22% — could not do
/// the job. A first-order tilt is not a notch, so it read as "the treble dipped"
/// rather than "the source went behind me".
const PINNA_FRONT_HZ: f32 = 9_000.0;
const PINNA_REAR_HZ: f32 = 6_500.0;
/// Sharpness of the notch. 1.2 puts its -3 dB points at about centre ÷ 1.45
/// and centre × 1.45 at full depth, which keeps even the rear notch under 1 dB
/// down at 3 kHz. A notch that bled into the presence region would dull vocals
/// every time the image swung behind the head, which is exactly the complaint
/// the previous front/back cue earned.
const PINNA_Q: f32 = 1.2;
/// The notch filter's damping, `1 / PINNA_Q` — and so also the factor that
/// scales its band-pass output to unity at the centre.
const PINNA_K: f32 = 1.0 / PINNA_Q;
/// Depth of the notch at full orbit depth, as a fraction of the band.
///
/// Real pinna notches are deeper than this. It is kept at 0.7 (about -10 dB)
/// because the centre frequency sweeps continuously with the orbit, and a
/// deeper moving notch stops reading as a direction and starts reading as a
/// wah pedal. Bounded below 1.0 regardless: at 1.0 the centre would be a
/// complete null, and beyond it the band would come back phase-inverted.
const MAX_PINNA_DEPTH: f32 = 0.7;

/// Longest early reflection, and therefore the room delay line length.
const MAX_ER_SECONDS: f32 = 0.050;
/// Early-reflection bed level at full orbit depth, relative to the direct
/// sound. Around -10 dB: enough to externalise, quiet enough that it reads as
/// space rather than as an effect.
const MAX_ROOM: f32 = 0.32;
/// Wall and air absorption on the reflected path. Reflections are duller than
/// the direct sound; without this the bed sounds like a slapback delay.
const ER_DAMPING_HZ: f32 = 4_500.0;

/// Early reflections for one ear, as (delay in seconds, gain).
///
/// The two patterns share no delay time, so the beds are decorrelated and add
/// width instead of a phantom centre. Gains fall roughly as 1/distance, as
/// reflections off a small room's surfaces do.
const ER_TAPS_L: [(f32, f32); 5] = [
    (0.0113, 0.70),
    (0.0179, 0.52),
    (0.0261, 0.40),
    (0.0357, 0.30),
    (0.0443, 0.22),
];
const ER_TAPS_R: [(f32, f32); 5] = [
    (0.0137, 0.66),
    (0.0213, 0.49),
    (0.0299, 0.37),
    (0.0391, 0.28),
    (0.0479, 0.21),
];

/// Summed *power* of one ear's taps, used to normalise the wet/dry mix.
///
/// Derived rather than written down so that editing a tap cannot silently
/// leave the mix law calibrated for the old pattern.
const fn tap_energy(taps: &[(f32, f32); 5]) -> f32 {
    let mut sum = 0.0;
    let mut i = 0;
    while i < taps.len() {
        sum += taps[i].1 * taps[i].1;
        i += 1;
    }
    sum
}
const ER_ENERGY: f32 = {
    let l = tap_energy(&ER_TAPS_L);
    let r = tap_energy(&ER_TAPS_R);
    if l > r { l } else { r }
};

/// Rounds a recursive filter's state to zero once it is far below audibility.
///
/// A recursive filter fed digital silence — a pause, the gap between two
/// tracks — decays geometrically toward zero and, without this, spends that
/// time in subnormal floats, which the CPU handles in microcode. Engaged on
/// silence, this unit measured four times its cost on music, on the thread
/// where a stall is a dropout.
fn flush_denormal(x: f32) -> f32 {
    if x.abs() < DENORMAL_FLOOR { 0.0 } else { x }
}

/// The three parameters as the listener has set them, read once per sample.
#[derive(Clone, Copy)]
struct Settings {
    pan: f32,
    rate: f32,
    depth: f32,
}

impl Settings {
    /// Whether anything is asked of the unit at all.
    fn engaged(self) -> bool {
        self.pan.abs() >= 0.001 || self.depth >= 0.001
    }

    fn orbiting(self) -> bool {
        self.depth > 0.001 && self.rate > 0.001
    }

    /// Where every amount is heading, with the orbit at `orbit_sin`.
    ///
    /// All zero when nothing is engaged, so that switching off is a glide to a
    /// stop rather than a cut.
    fn targets(self, orbit_sin: f32) -> Amounts {
        if !self.engaged() {
            return Amounts::default();
        }
        let orbiting = self.orbiting();
        let auto = if orbiting {
            orbit_sin * self.depth
        } else {
            0.0
        };
        let engagement = self.depth.max(self.pan.abs());
        Amounts {
            pan: (self.pan + auto).clamp(-1.0, 1.0),
            crossfeed: MAX_CROSSFEED
                * (MIN_CROSSFEED_SCALE + (1.0 - MIN_CROSSFEED_SCALE) * engagement),
            // Tied to orbit depth rather than to `engagement`, because a
            // listener nudging the balance control wants a balance control,
            // not a room; asking for the orbit is asking to be put somewhere.
            room: MAX_ROOM * self.depth,
            // Front and back only mean something while the source is going
            // round, so a stopped orbit fades the cue out rather than dropping
            // it.
            pinna: if orbiting {
                MAX_PINNA_DEPTH * self.depth
            } else {
                0.0
            },
        }
    }
}

/// How hard each part of the processing is driven.
///
/// Every sample the unit works out one of these for where the controls are
/// heading, moves a second one a little of the way toward it, and only ever
/// applies the second to the signal.
#[derive(Clone, Copy, Default)]
struct Amounts {
    /// Image position, -1 (left) to 1 (right): the static pan plus the orbit.
    pan: f32,
    /// Crossfeed gain, up to [`MAX_CROSSFEED`].
    crossfeed: f32,
    /// Early-reflection level, up to [`MAX_ROOM`].
    room: f32,
    /// Pinna notch depth, up to [`MAX_PINNA_DEPTH`].
    pinna: f32,
}

impl Amounts {
    /// Moves every amount one sample's worth of the way toward `target`.
    fn glide_toward(&mut self, target: Amounts, coeff: f32) {
        let glide = |from: f32, to: f32| flush_denormal(from + (to - from) * coeff);
        self.pan = glide(self.pan, target.pan);
        self.crossfeed = glide(self.crossfeed, target.crossfeed);
        self.room = glide(self.room, target.room);
        self.pinna = glide(self.pinna, target.pinna);
    }

    /// Whether every amount is close enough to zero that cutting the
    /// processed path off now could not be heard.
    fn has_settled(&self) -> bool {
        self.pan.abs() < SETTLED
            && self.crossfeed < SETTLED
            && self.room < SETTLED
            && self.pinna < SETTLED
    }
}

/// Coefficients of the trapezoidal state-variable filter the pinna notch is
/// cut with — the topology-preserving form, after Zavalishin and Simper.
///
/// The notch is the input minus a band-pass of itself, and this band-pass is
/// chosen for one property: at its own centre its response is exactly 1, real,
/// with no phase shift. So the notch there is exactly `1 - depth`, at every
/// sample rate, and that is also its deepest point.
///
/// The version before this built the band as the difference of two cascaded
/// one-pole low-passes, whose phase at the centre is not zero and moves with
/// sample rate. It solved for the gain that made the centre the right depth,
/// which got the centre right and left the rest of the shape to chance: at
/// 48 kHz the rear notch's real minimum sat at 5.4 kHz with 3 kHz already 5 dB
/// down, and at 96 kHz it had become a 41 dB hole at 4.5 kHz.
///
/// It also keeps the property the low-pass construction was chosen for. The
/// orbit retunes the notch every sample, and the trapezoidal state-variable
/// filter stays stable under modulation, which a direct-form biquad re-tuned
/// per sample does not guarantee.
#[derive(Clone, Copy)]
struct NotchCoeffs {
    a1: f32,
    a2: f32,
    a3: f32,
}

impl NotchCoeffs {
    /// `g` is the prewarped integrator gain, `tan(π·f/fs)`; see [`notch_g`].
    fn from_g(g: f32) -> Self {
        let a1 = 1.0 / (1.0 + g * (g + PINNA_K));
        let a2 = g * a1;
        Self { a1, a2, a3: g * a2 }
    }
}

/// Prewarped integrator gain for a notch centred on `centre_hz`.
///
/// The centre is kept clear of Nyquist, so a sample rate too low to hold it
/// puts the notch near the top of the band instead of asking `tan` for
/// infinity.
fn notch_g(centre_hz: f32, sample_rate: f32) -> f32 {
    (PI * centre_hz.min(sample_rate * 0.45) / sample_rate).tan()
}

#[derive(Clone)]
pub struct StereoSpatializer {
    pan: Shared,
    auto_pan_rate: Shared,
    auto_pan_depth: Shared,
    sample_rate: f32,

    phase: f32,
    /// Where every control has got to; see [`Amounts`].
    amounts: Amounts,
    smoothing_coeff: f32,
    /// Whether the unit is fully bypassed: every amount is exactly zero and
    /// the signal path is empty, so a sample can go straight through.
    at_rest: bool,

    /// One-pole coefficients, worked out once per sample rate rather than
    /// paying for an `exp` per filter per sample.
    shadow_alpha: f32,
    ild_alpha: f32,
    er_damping_alpha: f32,
    /// Prewarped notch gains at the two ends of its travel. The notch sweeps
    /// between them by interpolating the gain rather than the frequency,
    /// which keeps the sweep to a few multiplies per sample; the centre still
    /// moves monotonically between the two, within 1% of a straight line in
    /// hertz at 44.1 kHz and above.
    pinna_g_front: f32,
    pinna_g_rear: f32,

    /// One delay line per channel: the whole point is that they stay separate.
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    write_idx: usize,
    /// How many samples have been written to the delay lines since they were
    /// last emptied, up to their length.
    history: usize,

    /// One-pole states for the shadowing low-pass on each crossfeed path.
    shadow_l: f32,
    shadow_r: f32,
    /// One-pole states for each ear's *own* low band, which the crossfeed
    /// displaces rather than adds to.
    own_low_l: f32,
    own_low_r: f32,
    /// Band-split states for the frequency-dependent interaural level
    /// difference.
    ild_l: f32,
    ild_r: f32,
    /// State-variable filter states for each ear's pinna notch.
    pinna_l: [f32; 2],
    pinna_r: [f32; 2],

    /// Room delay lines, fed from the post-pan output of each ear.
    er_l: Vec<f32>,
    er_r: Vec<f32>,
    er_idx: usize,
    /// Absorption states for each ear's reflected path.
    er_damp_l: f32,
    er_damp_r: f32,
    /// `ER_TAPS_*` converted to sample offsets for the current sample rate.
    er_offsets_l: [usize; 5],
    er_offsets_r: [usize; 5],
}

impl StereoSpatializer {
    pub fn new(params: &AudioParams) -> Self {
        let mut s = Self {
            pan: params.pan.clone(),
            auto_pan_rate: params.auto_pan_rate.clone(),
            auto_pan_depth: params.auto_pan_depth.clone(),
            sample_rate: DEFAULT_SR as f32,
            phase: 0.0,
            amounts: Amounts::default(),
            smoothing_coeff: 0.0,
            at_rest: true,
            shadow_alpha: 0.0,
            ild_alpha: 0.0,
            er_damping_alpha: 0.0,
            pinna_g_front: 0.0,
            pinna_g_rear: 0.0,
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            write_idx: 0,
            history: 0,
            shadow_l: 0.0,
            shadow_r: 0.0,
            own_low_l: 0.0,
            own_low_r: 0.0,
            ild_l: 0.0,
            ild_r: 0.0,
            pinna_l: [0.0; 2],
            pinna_r: [0.0; 2],
            er_l: Vec::new(),
            er_r: Vec::new(),
            er_idx: 0,
            er_damp_l: 0.0,
            er_damp_r: 0.0,
            er_offsets_l: [0; 5],
            er_offsets_r: [0; 5],
        };
        s.reconfigure();
        s
    }

    fn reconfigure(&mut self) {
        let len = Ord::max((self.sample_rate * MAX_ITD_SECONDS).ceil() as usize + 4, 4);
        self.delay_l = vec![0.0; len];
        self.delay_r = vec![0.0; len];
        self.write_idx = 0;

        self.smoothing_coeff = Self::lowpass_alpha(CONTROL_SMOOTHING_HZ, self.sample_rate);
        self.shadow_alpha = Self::lowpass_alpha(SHADOW_CUTOFF_HZ, self.sample_rate);
        self.ild_alpha = Self::lowpass_alpha(ILD_SPLIT_HZ, self.sample_rate);
        self.er_damping_alpha = Self::lowpass_alpha(ER_DAMPING_HZ, self.sample_rate);

        // The room line has to hold the longest tap, plus a sample of slack so
        // the newest write is never the one being read back.
        let er_len = Ord::max((self.sample_rate * MAX_ER_SECONDS).ceil() as usize + 2, 4);
        self.er_l = vec![0.0; er_len];
        self.er_r = vec![0.0; er_len];
        self.er_idx = 0;
        // Clamped rather than asserted: a pathological sample rate should make
        // the room shallow, not panic in the audio callback.
        let to_offset =
            |seconds: f32| ((seconds * self.sample_rate).round() as usize).clamp(1, er_len - 1);
        self.er_offsets_l = std::array::from_fn(|i| to_offset(ER_TAPS_L[i].0));
        self.er_offsets_r = std::array::from_fn(|i| to_offset(ER_TAPS_R[i].0));

        self.pinna_g_front = notch_g(PINNA_FRONT_HZ, self.sample_rate);
        self.pinna_g_rear = notch_g(PINNA_REAR_HZ, self.sample_rate);
    }

    fn settings(&self) -> Settings {
        Settings {
            pan: self.pan.value().clamp(-1.0, 1.0),
            rate: self.auto_pan_rate.value().clamp(0.0, 2.0),
            depth: self.auto_pan_depth.value().clamp(0.0, 1.0),
        }
    }

    /// Constant-power balance, so sweeping across centre does not dip.
    fn balance_gains(pan: f32) -> (f32, f32) {
        let angle = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * TAU * 0.5; // 0..π/2
        let (sin, cos) = angle.sin_cos();
        (cos, sin)
    }

    fn read(line: &[f32], write_idx: usize, delay_samples: f32) -> f32 {
        let len = line.len();
        let max = (len - 2) as f32;
        let mut pos = write_idx as f32 - delay_samples.clamp(0.0, max);
        while pos < 0.0 {
            pos += len as f32;
        }
        let i0 = (pos.floor() as usize) % len;
        let i1 = (i0 + 1) % len;
        let frac = pos - pos.floor();
        line[i0] * (1.0 - frac) + line[i1] * frac
    }

    /// The coefficient [`Self::lowpass`] needs for `cutoff_hz`.
    fn lowpass_alpha(cutoff_hz: f32, sample_rate: f32) -> f32 {
        1.0 - (-TAU * cutoff_hz / sample_rate).exp()
    }

    /// One-pole low-pass: the head's shadow on the far ear, and every band
    /// split and absorption filter here besides.
    fn lowpass(state: &mut f32, sample: f32, alpha: f32) -> f32 {
        *state = flush_denormal(*state + alpha * (sample - *state));
        *state
    }

    /// Splits `sample` at [`ILD_SPLIT_HZ`] and attenuates only the high band.
    ///
    /// The broadband balance law is left to apply on top of this, so the low
    /// band comes through exactly as it did before this stage existed and only
    /// the high band gains the extra head shadow.
    fn hf_shadow(state: &mut f32, sample: f32, keep_high: f32, alpha: f32) -> f32 {
        let low = Self::lowpass(state, sample, alpha);
        low + (sample - low) * keep_high
    }

    /// One sample of the pinna notch: the input minus `depth` of its
    /// band-pass. See [`NotchCoeffs`].
    fn pinna_notch(state: &mut [f32; 2], sample: f32, coeffs: NotchCoeffs, depth: f32) -> f32 {
        let [ic1, ic2] = *state;
        let v3 = sample - ic2;
        let band = coeffs.a1 * ic1 + coeffs.a2 * v3;
        let low = ic2 + coeffs.a2 * ic1 + coeffs.a3 * v3;
        *state = [
            flush_denormal(2.0 * band - ic1),
            flush_denormal(2.0 * low - ic2),
        ];
        // `PINNA_K * band` is the band-pass scaled to unity at its centre.
        sample - depth * PINNA_K * band
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let settings = self.settings();
        let engaged = settings.engaged();

        // Nothing asked for and nothing still fading out: stay bit-transparent
        // rather than paying for a spatialiser that is not doing anything.
        if !engaged && self.at_rest {
            return (left, right);
        }

        if settings.orbiting() {
            self.phase = (self.phase + settings.rate / self.sample_rate).fract();
        }
        let (orbit_sin, orbit_cos) = (self.phase * TAU).sin_cos();
        self.amounts
            .glide_toward(settings.targets(orbit_sin), self.smoothing_coeff);

        // Switched off: the amounts are now gliding to zero, and the processed
        // path keeps running until they arrive. Cutting straight over to the
        // dry signal instead is a step from whatever the effect was doing to
        // nothing, which is a click every time.
        if !engaged && self.amounts.has_settled() {
            self.come_to_rest();
            return (left, right);
        }
        self.at_rest = false;

        let Amounts {
            pan,
            crossfeed,
            room,
            pinna,
        } = self.amounts;

        self.delay_l[self.write_idx] = left;
        self.delay_r[self.write_idx] = right;
        self.history = Ord::min(self.history + 1, self.delay_l.len());
        // No read reaches back past what has been written since the lines were
        // last emptied. Beyond that is silence, and a path that reads silence
        // until the signal arrives starts with a hard edge in the middle of a
        // waveform; holding the oldest sample instead starts it smoothly.
        let reach = (self.history - 1) as f32;

        // The ear further from the source hears it later.
        let itd = self.sample_rate * MAX_ITD_SECONDS;
        let direct_l = Self::read(
            &self.delay_l,
            self.write_idx,
            (pan.max(0.0) * itd).min(reach),
        );
        let direct_r = Self::read(
            &self.delay_r,
            self.write_idx,
            ((-pan).max(0.0) * itd).min(reach),
        );

        // Interaural level difference, the part the balance law cannot express.
        // The far ear loses its high band to the head; the near ear keeps
        // everything. At centre this is identity in both ears. Both band splits
        // run on every sample, so neither is holding stale state when the image
        // swings across and makes its ear the far one.
        let hf_shadow = pan.abs() * MAX_HF_SHADOW;
        let (keep_l, keep_r) = if pan > 0.0 {
            (1.0 - hf_shadow, 1.0) // source right: the left ear is shadowed
        } else {
            (1.0, 1.0 - hf_shadow)
        };
        let direct_l = Self::hf_shadow(&mut self.ild_l, direct_l, keep_l, self.ild_alpha);
        let direct_r = Self::hf_shadow(&mut self.ild_r, direct_r, keep_r, self.ild_alpha);

        // Crossfeed carries the *opposite* channel, delayed by the full head
        // width and shadowed. Because it is low-passed, separation above the
        // cutoff is untouched — which is what preserves the stereo image.
        let cross_delay = itd.min(reach);
        let cross_r = Self::lowpass(
            &mut self.shadow_l,
            Self::read(&self.delay_r, self.write_idx, cross_delay),
            self.shadow_alpha,
        );
        let cross_l = Self::lowpass(
            &mut self.shadow_r,
            Self::read(&self.delay_l, self.write_idx, cross_delay),
            self.shadow_alpha,
        );
        // A shuffler, not a blend: each ear gives up as much of its *own* low
        // band as it receives of the other's, so the crossfeed moves bass
        // toward the centre instead of adding it.
        //
        // The obvious alternative — mix the crossfeed in and divide by
        // `1 + crossfeed` — was what this did before, and it is wrong in a way
        // that is easy to miss and easy to hear. The crossfeed is low-passed,
        // so it only adds energy below the shadow cutoff, but that
        // normalisation is broadband: it paid for two octaves of added bass by
        // attenuating the entire spectrum, and engaging the spatialiser
        // measured 2.1 dB quieter than bypassing it. An effect that turns the
        // music down loses every A/B it is ever given, however good the rest
        // of it is.
        let own_low_l = Self::lowpass(&mut self.own_low_l, direct_l, self.shadow_alpha);
        let own_low_r = Self::lowpass(&mut self.own_low_r, direct_r, self.shadow_alpha);
        let mut out_l = direct_l + (cross_r - own_low_l) * crossfeed;
        let mut out_r = direct_r + (cross_l - own_low_r) * crossfeed;

        self.write_idx = (self.write_idx + 1) % self.delay_l.len();

        // Front/back is cued by the pinna notch sliding down the spectrum as
        // the source passes behind the head. Applied per channel to its own
        // signal, so unlike a rear-centre cue it costs no separation.
        if pinna > SETTLED {
            // 0 in front, 1 directly behind. `phase` is the orbit angle, so
            // this stays coherent with the ITD and the balance rather than
            // being an independent wobble. Phase 0 is straight ahead, so the
            // orbit starts — and restarts after every seek — in front of the
            // listener rather than behind them.
            let behind = (1.0 - orbit_cos) * 0.5;
            let g = self.pinna_g_front + (self.pinna_g_rear - self.pinna_g_front) * behind;
            let coeffs = NotchCoeffs::from_g(g);
            out_l = Self::pinna_notch(&mut self.pinna_l, out_l, coeffs, pinna);
            out_r = Self::pinna_notch(&mut self.pinna_r, out_r, coeffs, pinna);
        }

        let (gain_l, gain_r) = Self::balance_gains(pan);
        let out_l = out_l * gain_l * SQRT_2;
        let out_r = out_r * gain_r * SQRT_2;

        // The room is fed the output at its current level, so the reflections
        // carry whatever level the room had when that sound was made. Turning
        // it up therefore fills it from silence, where scaling the reflections
        // on the way out would open onto an empty line whose first reflection
        // arrives 11 ms later as a hard edge in the middle of a waveform; and
        // turning it down lets the reflections already in flight die away
        // instead of cutting them. The line is fed on every engaged sample,
        // zeros included, because a line written only while it is audible is
        // stale the instant it becomes audible again — the same trap as
        // bypassing without clearing, one level down.
        let (wet_l, wet_r) = self.early_reflections(out_l * room, out_r * room);
        // Equal-power, using the bed's own energy sum: the taps are
        // decorrelated, so their contributions add in power, not amplitude.
        // Normalising by the amplitude sum instead would make the mix quieter
        // the more room was asked for.
        let norm = 1.0 / (1.0 + room * room * ER_ENERGY).sqrt();
        ((out_l + wet_l) * norm, (out_r + wet_r) * norm)
    }

    /// Drops into bypass: empties the signal path and pins every amount at
    /// exactly zero.
    ///
    /// Every delay line here is written only while the effect is engaged, so a
    /// line left full during bypass is a recording of whatever was playing
    /// when the effect was switched off, waiting to be read straight back out
    /// the moment it is switched on again — the interaural lines feed the
    /// crossfeed, the crossfeed feeds the room, and the room holds it for a
    /// further 50 ms, so leaving any one of them behind is audible. Clearing
    /// here, once, on the way in keeps the bypass itself free.
    fn come_to_rest(&mut self) {
        self.clear_signal_state();
        self.amounts = Amounts::default();
        self.at_rest = true;
    }

    /// Empties every delay line and filter state, leaving the orbit phase and
    /// the amounts alone.
    ///
    /// Those are position, not signal, and each caller decides where they
    /// should be.
    fn clear_signal_state(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.history = 0;
        self.shadow_l = 0.0;
        self.shadow_r = 0.0;
        self.own_low_l = 0.0;
        self.own_low_r = 0.0;
        self.ild_l = 0.0;
        self.ild_r = 0.0;
        self.pinna_l = [0.0; 2];
        self.pinna_r = [0.0; 2];
        self.er_l.fill(0.0);
        self.er_r.fill(0.0);
        self.er_idx = 0;
        self.er_damp_l = 0.0;
        self.er_damp_r = 0.0;
    }

    /// Reads both ears' reflection beds and advances the room line.
    ///
    /// Each ear reads only its own line: a bed that crossfed would reintroduce
    /// the unbudgeted channel-to-channel path this whole unit was rewritten to
    /// eliminate.
    fn early_reflections(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let len = self.er_l.len();
        self.er_l[self.er_idx] = in_l;
        self.er_r[self.er_idx] = in_r;

        let mut wet_l = 0.0;
        let mut wet_r = 0.0;
        for i in 0..ER_TAPS_L.len() {
            // `+ len` keeps the subtraction in range without a signed cast;
            // the offsets are clamped to `1..len-1` when the line is sized.
            let idx_l = (self.er_idx + len - self.er_offsets_l[i]) % len;
            let idx_r = (self.er_idx + len - self.er_offsets_r[i]) % len;
            wet_l += self.er_l[idx_l] * ER_TAPS_L[i].1;
            wet_r += self.er_r[idx_r] * ER_TAPS_R[i].1;
        }

        self.er_idx = (self.er_idx + 1) % len;

        (
            Self::lowpass(&mut self.er_damp_l, wet_l, self.er_damping_alpha),
            Self::lowpass(&mut self.er_damp_r, wet_r, self.er_damping_alpha),
        )
    }
}

impl AudioUnit for StereoSpatializer {
    fn reset(&mut self) {
        self.phase = 0.0;
        let settings = self.settings();
        self.clear_signal_state();
        // A fresh stream has no earlier output to be continuous with, so the
        // image starts where it is heading rather than gliding there. The room
        // is the exception: its line has just been emptied, and it fills from
        // silence for the reason given where it is fed.
        self.amounts = Amounts {
            room: 0.0,
            ..settings.targets(0.0)
        };
        self.at_rest = !settings.engaged();
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.reconfigure();
        self.reset();
    }

    fn tick(&mut self, input: &[f32], output: &mut [f32]) {
        let (l, r) = self.process_sample(input[0], input[1]);
        output[0] = l;
        output[1] = r;
    }

    fn process(&mut self, size: usize, input: &BufferRef, output: &mut BufferMut) {
        for i in 0..size {
            let (l, r) = self.process_sample(input.channel_f32(0)[i], input.channel_f32(1)[i]);
            output.channel_f32_mut(0)[i] = l;
            output.channel_f32_mut(1)[i] = r;
        }
    }

    fn inputs(&self) -> usize {
        2
    }

    fn outputs(&self) -> usize {
        2
    }

    fn route(&mut self, input: &SignalFrame, _frequency: f64) -> SignalFrame {
        let mut output = SignalFrame::new(2);
        output.set(0, input.at(0).combine_nonlinear(input.at(1), 0.0));
        output.set(1, input.at(1).combine_nonlinear(input.at(0), 0.0));
        output
    }

    fn get_id(&self) -> u64 {
        0x05a7_1a11_05fa_71a1
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    fn spatializer(setup: impl Fn(&AudioParams)) -> StereoSpatializer {
        let params = AudioParams::new();
        setup(&params);
        let mut s = StereoSpatializer::new(&params);
        s.set_sample_rate(48_000.0);
        s
    }

    /// The frequencies making up one probe band: eleven tones spanning about
    /// 230 Hz around `centre`.
    ///
    /// A band rather than a single tone because the room is a comb filter, and
    /// a comb has a different gain at 400 Hz than it does at 700 Hz. Probing
    /// each band at one frequency therefore measures where those two
    /// frequencies happen to fall on the comb's ripple as much as it measures
    /// channel separation — which is not the property under test, and would
    /// make these tests fail or pass on a change to a reflection delay that
    /// cannot affect separation at all. Summing power across a band wide enough
    /// to cover several ripple periods averages the comb out.
    fn probe_band(centre: f32) -> [f32; 11] {
        std::array::from_fn(|i| centre + (i as f32 - 5.0) * 23.0)
    }

    /// Bleed of the `fb` band (right-channel-only) into the left output, in dB
    /// relative to the wanted left signal in the `fa` band. 0 dB would be fully
    /// mono.
    ///
    /// Measured with a static pan. The orbiting auto-pan sweeps the ITD, which
    /// Doppler-shifts a steady tone by around a hertz and smears it out of a
    /// narrow analysis bin — that would under-read the wanted signal and make
    /// the effect look far worse than it is. Width under orbit is covered by
    /// `orbiting_does_not_narrow_the_image` instead, which is immune to it.
    fn channel_bleed_db(depth: f32, fa: f32, fb: f32) -> f32 {
        const SR: f32 = 48_000.0;
        let band_a = probe_band(fa);
        let band_b = probe_band(fb);

        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(depth);
            p.auto_pan_rate.set_value(0.0);
            p.pan.set_value(depth);
        });

        // Fixed irrational phase offsets so the tones in a band do not all peak
        // together, which would give the limiterless chain a needlessly spiky
        // input, while keeping the signal exactly reproducible run to run.
        let tone = |band: &[f32; 11], t: f32| -> f32 {
            band.iter()
                .enumerate()
                .map(|(k, &f)| (t * f * TAU + k as f32 * 1.7).sin())
                .sum::<f32>()
                * (0.25 / 11.0f32.sqrt())
        };

        let mut out_l = Vec::with_capacity(48_000);
        for i in 0..48_000 {
            let t = i as f32 / SR;
            let mut o = [0.0; 2];
            s.tick(&[tone(&band_a, t), tone(&band_b, t)], &mut o);
            out_l.push(o[0]);
        }

        // Hann-windowed Goertzel. Without the window, a rectangular window's
        // sidelobes decay slowly enough that the 400 Hz band would leak into
        // the 700 Hz measurement at around the level being measured.
        let band_power = |sig: &[f32], band: &[f32; 11]| -> f64 {
            let s = &sig[sig.len() / 3..];
            let n = s.len() as f64;
            band.iter()
                .map(|&f| {
                    let (mut re, mut im) = (0.0f64, 0.0f64);
                    for (i, &v) in s.iter().enumerate() {
                        let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (n - 1.0)).cos();
                        let ph = (i as f64) / SR as f64 * f as f64 * std::f64::consts::TAU;
                        re += v as f64 * w * ph.cos();
                        im += v as f64 * w * ph.sin();
                    }
                    (re * re + im * im) / (n * n)
                })
                .sum()
        };
        10.0 * (band_power(&out_l, &band_b).max(1e-24) / band_power(&out_l, &band_a).max(1e-24))
            .log10() as f32
    }

    /// Ratio of side (stereo-only) to mid (shared) energy, in dB.
    fn side_over_mid_db(out: &[(f32, f32)]) -> f32 {
        let skip = out.len() / 4;
        let (mut m, mut s) = (0.0f64, 0.0f64);
        for &(l, r) in &out[skip..] {
            m += (((l + r) * 0.5) as f64).powi(2);
            s += (((l - r) * 0.5) as f64).powi(2);
        }
        20.0 * ((s / m).sqrt() as f32).max(1e-12).log10()
    }

    #[test]
    fn is_transparent_when_nothing_is_engaged() {
        // Pan centred and no orbit: the spatialiser must pass audio through
        // untouched rather than run a delay line for no reason.
        let mut s = spatializer(|_| {});
        let mut out = [0.0, 0.0];
        s.tick(&[0.8, -0.2], &mut out);
        assert_close(out[0], 0.8);
        assert_close(out[1], -0.2);
    }

    #[test]
    fn centre_pan_does_not_attenuate() {
        let mut s = spatializer(|p| p.pan.set_value(0.0));
        let mut out = [0.0, 0.0];
        s.tick(&[0.5, 0.5], &mut out);
        assert_close(out[0], 0.5);
        assert_close(out[1], 0.5);
    }

    #[test]
    fn hard_pan_moves_the_image() {
        let mut s = spatializer(|p| p.pan.set_value(1.0));
        let mut out = [0.0, 0.0];
        for _ in 0..8_000 {
            s.tick(&[0.5, 0.5], &mut out);
        }
        assert!(
            out[1].abs() > out[0].abs() * 4.0,
            "hard right should be dominated by the right channel, got {out:?}"
        );
    }

    #[test]
    fn the_midrange_keeps_its_separation() {
        // This is the band the stereo image actually lives in. The old
        // mono-summing implementation measured about -7 dB of bleed here at
        // full depth, which is very nearly mono.
        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            let mid = channel_bleed_db(depth, 400.0, 3_000.0);
            let high = channel_bleed_db(depth, 400.0, 8_000.0);
            assert!(mid < -20.0, "depth {depth}: {mid:.1} dB of bleed at 3 kHz");
            assert!(
                high < -30.0,
                "depth {depth}: {high:.1} dB of bleed at 8 kHz"
            );
        }
    }

    #[test]
    fn crossfeed_stays_within_its_budget_and_is_off_at_zero() {
        // Below the shadow cutoff the ears genuinely share content — that is
        // what externalises the image on headphones — but it must stay bounded,
        // and must not engage at all when the effect is off.
        assert!(
            channel_bleed_db(0.0, 400.0, 700.0) < -60.0,
            "with the effect off the channels must not touch"
        );
        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            let low = channel_bleed_db(depth, 400.0, 700.0);
            // Bounded on both sides. Too much and the bottom end is mono, which
            // is the failure the whole rewrite exists to prevent; too little and
            // there is no crossfeed to speak of, which is a silent regression a
            // ceiling alone would happily accept.
            //
            // The ceiling is -8.5 dB rather than the -12 this asserted before
            // the crossfeed became a shuffler. That is not a relaxation to
            // accommodate the change: reducing low-frequency separation is
            // precisely what crossfeed is *for*, analogue crossfeed circuits
            // target 6-10 dB of it, and the old figure described what the
            // previous mix law happened to leave behind rather than anything
            // anyone wanted. Mono summing reads about -2 dB here, so the bound
            // still has several dB of room to catch it.
            assert!(
                (-30.0..-8.5).contains(&low),
                "depth {depth}: {low:.1} dB of crossfeed is outside the intended range"
            );
        }
    }

    #[test]
    fn orbiting_does_not_narrow_the_image() {
        // Decorrelated input, so side and mid energy start out comparable.
        // Whatever the effect does to the image, it must not shrink the side
        // channel — that is the "background goes dead" failure.
        let (mut a, mut b) = (12345u32, 987654u32);
        let rnd = |s: &mut u32| {
            *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            ((*s >> 8) as f32 / 8_388_608.0) - 1.0
        };
        let input: Vec<(f32, f32)> = (0..96_000)
            .map(|_| (rnd(&mut a) * 0.3, rnd(&mut b) * 0.3))
            .collect();
        let reference = side_over_mid_db(&input);

        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            for rate in [0.0f32, 0.15] {
                let mut s = spatializer(|p| {
                    p.auto_pan_depth.set_value(depth);
                    p.auto_pan_rate.set_value(rate);
                });
                let out: Vec<(f32, f32)> = input
                    .iter()
                    .map(|&(l, r)| {
                        let mut o = [0.0; 2];
                        s.tick(&[l, r], &mut o);
                        (o[0], o[1])
                    })
                    .collect();
                let width = side_over_mid_db(&out);
                assert!(
                    width > reference - 3.0,
                    "depth {depth} rate {rate}: width fell from {reference:.2} to {width:.2} dB"
                );
            }
        }
    }

    #[test]
    fn auto_pan_actually_moves_the_image() {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(0.9);
            p.auto_pan_rate.set_value(1.0);
        });
        let mut min_ratio = f32::MAX;
        let mut max_ratio = f32::MIN;
        let mut out = [0.0, 0.0];
        for i in 0..48_000 {
            let v = (i as f32 / 48_000.0 * 500.0 * TAU).sin() * 0.3;
            s.tick(&[v, v], &mut out);
            if i > 8_000 && out[1].abs() > 1e-6 {
                let ratio = out[0].abs() / out[1].abs();
                min_ratio = min_ratio.min(ratio);
                max_ratio = max_ratio.max(ratio);
            }
        }
        assert!(
            max_ratio / min_ratio > 2.0,
            "the image should sweep, L/R ratio spanned {min_ratio}..{max_ratio}"
        );
    }

    #[test]
    fn stays_finite_and_bounded_under_full_settings() {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(1.0);
            p.auto_pan_rate.set_value(2.0);
            p.pan.set_value(-1.0);
        });
        let mut out = [0.0, 0.0];
        for i in 0..48_000 {
            let v = (i as f32 / 48_000.0 * 100.0 * TAU).sin();
            s.tick(&[v, -v], &mut out);
            assert!(out[0].is_finite() && out[1].is_finite());
            assert!(
                out[0].abs() < 4.0 && out[1].abs() < 4.0,
                "runaway gain: {out:?}"
            );
        }
    }

    // ── Interaural level difference ───────────────────────

    /// Energy of each output channel over a steady tone, in dB.
    fn ild_db(pan: f32, f: f32) -> f32 {
        const SR: f32 = 48_000.0;
        let mut s = spatializer(|p| p.pan.set_value(pan));
        let n = 24_000;
        let mut out = [0.0; 2];
        let (mut el, mut er) = (0.0f64, 0.0f64);
        for i in 0..n {
            let v = (i as f32 / SR * f * TAU).sin() * 0.3;
            s.tick(&[v, v], &mut out);
            if i > n / 3 {
                el += (out[0] as f64).powi(2);
                er += (out[1] as f64).powi(2);
            }
        }
        10.0 * (er / el.max(1e-18)).log10() as f32
    }

    #[test]
    fn the_level_difference_grows_with_frequency() {
        // The defining property of a real head, and the one a plain balance law
        // cannot express: a head is transparent to bass and opaque to treble,
        // so the far ear loses far more of the top than of the bottom. A
        // frequency-flat level difference is what a mixing-desk pan pot
        // produces, and it is heard as a source moving *inside* the head rather
        // than around it.
        for pan in [0.3f32, 0.6] {
            let low = ild_db(pan, 200.0);
            let high = ild_db(pan, 8_000.0);
            assert!(
                high - low > 3.0,
                "pan {pan}: level difference barely moved with frequency \
                 ({low:.1} dB at 200 Hz vs {high:.1} dB at 8 kHz)"
            );
        }
    }

    // ── Pinna / front-back cue ────────────────────────────

    /// Sample rates the notch is checked at: the common ones, up to the hi-res
    /// rates where the notch before this one came apart.
    const RATES: [f32; 5] = [44_100.0, 48_000.0, 88_200.0, 96_000.0, 192_000.0];

    /// Gain of the notch alone at `f`, in dB, centred on `centre` at full
    /// depth.
    ///
    /// Read off the impulse response rather than by playing a tone through it:
    /// with the centre held still the notch is a fixed linear filter, so the
    /// transform of its impulse response is its exact frequency response.
    fn notch_db(centre: f32, f: f32, sample_rate: f32) -> f32 {
        let coeffs = NotchCoeffs::from_g(notch_g(centre, sample_rate));
        let mut state = [0.0; 2];
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for n in 0..2_048 {
            let impulse = if n == 0 { 1.0 } else { 0.0 };
            let h = StereoSpatializer::pinna_notch(&mut state, impulse, coeffs, MAX_PINNA_DEPTH);
            let ph = n as f64 * f as f64 / sample_rate as f64 * std::f64::consts::TAU;
            re += h as f64 * ph.cos();
            im -= h as f64 * ph.sin();
        }
        10.0 * (re * re + im * im).max(1e-24).log10() as f32
    }

    #[test]
    fn the_notch_removes_the_fraction_of_the_band_it_says_it_does() {
        // `depth` is meant to mean "this fraction of the band is removed", so
        // depth 0.7 must measure as -10.5 dB at the centre, at every sample
        // rate. Get that wrong and every other constant here is being tuned
        // against a notch that is quietly a different depth than it claims.
        let expected = 20.0 * (1.0 - MAX_PINNA_DEPTH).log10();
        for sr in RATES {
            for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
                let got = notch_db(centre, centre, sr);
                assert!(
                    (got - expected).abs() < 0.1,
                    "sr {sr}, centre {centre}: notch is {got:.2} dB, expected {expected:.2} dB"
                );
            }
        }
    }

    #[test]
    fn the_notch_is_deepest_where_it_claims_to_be() {
        // Being the right depth at the centre is not enough. The notch before
        // this one was solved to be exactly -10.5 dB at its nominal centre and
        // had its real minimum somewhere else entirely: at 96 kHz the rear
        // notch was a 41 dB hole at 4.5 kHz. A notch in the wrong place is a
        // different cue, and a deep one that moves is a wah pedal.
        let expected = 20.0 * (1.0 - MAX_PINNA_DEPTH).log10();
        for sr in RATES {
            for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
                // Half an octave either side, in 1% steps.
                let (deepest, at) = (0..=70)
                    .map(|i| centre * 0.7 * 1.01f32.powi(i))
                    .map(|f| (notch_db(centre, f, sr), f))
                    .fold((f32::MAX, 0.0), |a, b| if b.0 < a.0 { b } else { a });
                assert!(
                    (at / centre - 1.0).abs() < 0.02,
                    "sr {sr}, centre {centre}: the deepest point is at {at:.0} Hz"
                );
                assert!(
                    deepest > expected - 0.1,
                    "sr {sr}, centre {centre}: {deepest:.1} dB deep, more than the \
                     {expected:.1} dB it claims"
                );
            }
        }
    }

    #[test]
    fn the_notch_leaves_the_midrange_and_presence_alone() {
        // The whole reason the previous front/back cues were unusable: they
        // dulled vocals and guitars every time the image swung behind the head.
        // A cue the listener hears as "the tone changed" is not a direction
        // cue. 3 kHz is where that is heard first, and where the notch before
        // this one was already 5 dB down.
        for sr in RATES {
            for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
                for (f, budget) in [
                    (200.0f32, 0.1f32),
                    (500.0, 0.1),
                    (1_000.0, 0.2),
                    (3_000.0, 1.0),
                ] {
                    let g = notch_db(centre, f, sr);
                    assert!(
                        g.abs() < budget,
                        "sr {sr}, centre {centre}: {g:.2} dB at {f} Hz"
                    );
                }
            }
        }
    }

    #[test]
    fn front_and_back_are_spectrally_distinguishable() {
        // Front/back is the one direction pair that interaural time and level
        // differences cannot resolve at all — both ears hear a source in front
        // and the same source behind identically. The notch moving is the only
        // thing carrying that information, so the two positions must produce
        // genuinely different spectra, not a token difference.
        for sr in RATES {
            for f in [PINNA_REAR_HZ, PINNA_FRONT_HZ] {
                let apart =
                    (notch_db(PINNA_FRONT_HZ, f, sr) - notch_db(PINNA_REAR_HZ, f, sr)).abs();
                assert!(
                    apart > 5.0,
                    "sr {sr}: front and back differ by only {apart:.1} dB at {f} Hz"
                );
            }
        }
    }

    #[test]
    fn the_notch_never_boosts() {
        // A notch that comes out louder anywhere is a peak, not a notch. The
        // normalised band-pass makes that impossible by construction; this is
        // here so it stays impossible if the construction changes.
        for sr in RATES {
            for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
                for i in 0..70 {
                    let f = 50.0 * 1.1f32.powi(i);
                    if f < sr * 0.49 {
                        let g = notch_db(centre, f, sr);
                        assert!(
                            g < 0.01,
                            "sr {sr}, centre {centre}: {f:.0} Hz is boosted by {g:.2} dB"
                        );
                    }
                }
            }
        }
    }

    /// Level of a tone at `f` in the left output with the orbit parked at
    /// `phase` (0 in front, 0.5 behind), relative to the same tone with the
    /// orbit stopped, in dB.
    ///
    /// Stopping the orbit changes nothing but the notch: the depth — and with
    /// it the room and the crossfeed — is the same either way, and parked dead
    /// ahead or dead behind the source is centred either way. So the ratio is
    /// the notch's own response, as the real signal path applies it.
    fn orbit_notch_db(phase: f32, f: f32) -> f32 {
        const SR: f32 = 48_000.0;
        let level = |rate: f32| {
            let mut s = spatializer(|p| {
                p.auto_pan_depth.set_value(1.0);
                p.auto_pan_rate.set_value(rate);
            });
            s.phase = phase;
            let mut out = [0.0; 2];
            let left: Vec<f32> = (0..24_000)
                .map(|i| {
                    let v = (i as f32 / SR * f * TAU).sin() * 0.3;
                    s.tick(&[v, v], &mut out);
                    out[0]
                })
                .collect();
            // A whole number of cycles of both probe tones, so neither leaks.
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &v) in left[8_064..].iter().enumerate() {
                let ph = i as f64 / SR as f64 * f as f64 * std::f64::consts::TAU;
                re += v as f64 * ph.cos();
                im += v as f64 * ph.sin();
            }
            (re * re + im * im).sqrt()
        };
        // Just fast enough to count as orbiting: the source moves by under a
        // thousandth of a turn while it is measured.
        20.0 * (level(0.0015) / level(0.0)).log10() as f32
    }

    #[test]
    fn the_orbit_puts_the_notch_where_the_source_is() {
        // The tests above exercise the notch on its own. This checks that the
        // signal path really applies it, at the depth it claims, and that it
        // follows the orbit: the front notch with the source dead ahead, the
        // rear one with it dead behind.
        let expected = 20.0 * (1.0 - MAX_PINNA_DEPTH).log10();
        let ahead_front = orbit_notch_db(0.0, PINNA_FRONT_HZ);
        let ahead_rear = orbit_notch_db(0.0, PINNA_REAR_HZ);
        let behind_front = orbit_notch_db(0.5, PINNA_FRONT_HZ);
        let behind_rear = orbit_notch_db(0.5, PINNA_REAR_HZ);
        assert!(
            (ahead_front - expected).abs() < 1.0,
            "source ahead: {ahead_front:.1} dB at {PINNA_FRONT_HZ} Hz, expected about {expected:.1}"
        );
        assert!(
            (behind_rear - expected).abs() < 1.0,
            "source behind: {behind_rear:.1} dB at {PINNA_REAR_HZ} Hz, expected about {expected:.1}"
        );
        assert!(
            ahead_rear - behind_rear > 4.0 && behind_front - ahead_front > 4.0,
            "the notch did not follow the source: ahead {ahead_front:.1}/{ahead_rear:.1} dB, \
             behind {behind_front:.1}/{behind_rear:.1} dB (front/rear frequency)"
        );
    }

    // ── The room ──────────────────────────────────────────

    /// Impulse response of the spatialiser to a left-only impulse, as
    /// (left, right) energy in the direct window and in the reflection window.
    ///
    /// The windows are chosen from the tap layout: every reflection lands after
    /// 11 ms, and everything before 5 ms is direct sound plus the crossfeed's
    /// short low-passed tail. The impulse comes after half a second of
    /// silence, by which time the room has filled to its full level.
    fn impulse_windows(depth: f32, pan: f32) -> ((f64, f64), (f64, f64)) {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(depth);
            p.auto_pan_rate.set_value(0.0);
            p.pan.set_value(pan);
        });
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let mut out = [0.0; 2];
        for _ in 0..24_000 {
            s.tick(&[0.0, 0.0], &mut out);
        }
        for i in 0..6_000 {
            s.tick(&[if i == 0 { 1.0 } else { 0.0 }, 0.0], &mut out);
            l.push(out[0]);
            r.push(out[1]);
        }
        let energy = |v: &[f32], a: usize, b: usize| {
            v[a..b].iter().map(|x| (*x as f64).powi(2)).sum::<f64>()
        };
        // 0..5 ms and 5..50 ms at 48 kHz.
        (
            (energy(&l, 0, 240), energy(&r, 0, 240)),
            (energy(&l, 240, 2_400), energy(&r, 240, 2_400)),
        )
    }

    #[test]
    fn the_room_exists_and_sits_behind_the_direct_sound() {
        // Externalisation is what the room is for, so it has to actually be
        // there — but a room louder than the source is a delay effect, not a
        // space.
        let ((direct, _), (room, _)) = impulse_windows(1.0, 0.0);
        let ratio = 10.0 * (room / direct).log10();
        assert!(
            (-25.0..-6.0).contains(&ratio),
            "reflections sit {ratio:.1} dB below the direct sound, which is not a room"
        );
    }

    #[test]
    fn the_room_is_ipsilateral() {
        // Each ear's reflections are built from that ear's own signal. If the
        // bed crossfed instead, a left-only impulse would arrive in the right
        // ear at full strength in the reflection window, and the room would be
        // a second unbudgeted path for one channel's content to reach the other
        // ear — the image collapse this unit was rewritten to fix, reappearing
        // by a new route and audible only as "the effect makes it mono-ish".
        let (_, (room_l, room_r)) = impulse_windows(1.0, 0.0);
        let bleed = 10.0 * (room_r / room_l.max(1e-30)).log10();
        assert!(
            bleed < -12.0,
            "the reflection window carries {bleed:.1} dB of the other channel — the bed is crossfeeding"
        );
    }

    #[test]
    fn the_balance_control_does_not_put_the_listener_in_a_room() {
        // Panning is a mixing decision; orbiting is a request to be placed
        // somewhere. Someone nudging the balance slider must not silently get
        // reverb they did not ask for.
        let ((direct, _), (room, _)) = impulse_windows(0.0, 0.7);
        let ratio = 10.0 * (room / direct.max(1e-30)).log10();
        assert!(
            ratio < -40.0,
            "a static pan produced reflections only {ratio:.1} dB down"
        );
    }

    #[test]
    fn engaging_the_effect_is_not_a_loudness_change() {
        // The room adds energy, so its mix law has to give it back. If it does
        // not, the spatialiser reads as "louder" in an A/B rather than
        // "wider", which is the oldest way to make an effect seem better than
        // it is.
        let (mut a, mut b) = (12_345u32, 987_654u32);
        let rnd = |s: &mut u32| {
            *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            ((*s >> 8) as f32 / 8_388_608.0) - 1.0
        };
        let input: Vec<(f32, f32)> = (0..96_000)
            .map(|_| (rnd(&mut a) * 0.3, rnd(&mut b) * 0.3))
            .collect();

        let rms = |depth: f32| {
            let mut s = spatializer(|p| {
                p.auto_pan_depth.set_value(depth);
                p.auto_pan_rate.set_value(0.0);
            });
            let mut out = [0.0; 2];
            let mut sum = 0.0f64;
            for (i, &(l, r)) in input.iter().enumerate() {
                s.tick(&[l, r], &mut out);
                if i > input.len() / 4 {
                    sum += (out[0] as f64).powi(2) + (out[1] as f64).powi(2);
                }
            }
            10.0 * (sum / input.len() as f64).max(1e-30).log10()
        };

        let delta = rms(1.0) - rms(0.0);
        assert!(
            delta.abs() < 2.0,
            "turning the room on changed the level by {delta:.2} dB"
        );
    }

    // ── Transitions ───────────────────────────────────────

    /// Largest second difference in either channel over `range`.
    ///
    /// A click is a discontinuity, and a discontinuity is a spike in the second
    /// difference that a smooth signal does not have. Measuring that rather
    /// than the first difference keeps this blind to slow gain changes — the
    /// image sweeping, the balance gliding — which change the output's slope
    /// without being anything anyone would hear as a click.
    fn roughness(out: &[(f32, f32)], range: std::ops::Range<usize>) -> f32 {
        let d2 = |a: f32, b: f32, c: f32| (c - 2.0 * b + a).abs();
        out[range]
            .windows(3)
            .map(|w| d2(w[0].0, w[1].0, w[2].0).max(d2(w[0].1, w[1].1, w[2].1)))
            .fold(0.0, f32::max)
    }

    /// Plays a steady tone, applies `change` half a second in, and returns how
    /// much rougher the output gets across the change than in the steadier of
    /// the stretches either side of it.
    ///
    /// The change lands on a peak of the tone. Anything that opens onto an
    /// empty delay line starts with a step the size of the signal at that
    /// moment, so a change at a zero crossing would hide exactly the failure
    /// this is looking for.
    fn click_ratio(setup: impl Fn(&AudioParams), change: impl Fn(&StereoSpatializer)) -> f32 {
        const SR: usize = 48_000;
        const AT: usize = SR / 2 + 80; // a quarter-cycle past a zero crossing
        let mut s = spatializer(setup);
        let tone = |i: usize| (i as f32 / SR as f32 * 150.0 * TAU).sin() * 0.5;
        let mut o = [0.0; 2];
        let out: Vec<(f32, f32)> = (0..2 * SR)
            .map(|i| {
                if i == AT {
                    change(&s);
                }
                s.tick(&[tone(i), tone(i) * 0.7], &mut o);
                (o[0], o[1])
            })
            .collect();
        let before = roughness(&out, SR / 4..AT);
        let across = roughness(&out, AT - 2..SR);
        let after = roughness(&out, 7 * SR / 4..2 * SR);
        across / before.max(after)
    }

    #[test]
    fn changing_a_control_is_a_glide_not_a_click() {
        // Every control scales something in the signal path, so a setting that
        // moves in one step — a keypress, a preset, a reset — is a gain step
        // unless the unit smooths it. Measured here before the controls were
        // smoothed, every one of these was between 20 and 1,800 times rougher
        // across the change than either side of it.
        //
        // The bound is 4 rather than 1 because a glide is not perfectly smooth
        // either: it starts at its fastest, so the gain bends where the change
        // lands, and at the peak of a full-scale tone that bend reads at about
        // twice the tone's own roughness. That is a corner in a gain, not a
        // step in the signal, and nothing anyone can hear.
        let orbit = |p: &AudioParams| {
            p.auto_pan_depth.set_value(0.9);
            p.auto_pan_rate.set_value(0.1);
        };
        let cases = [
            (
                "switching the orbit off",
                click_ratio(orbit, |s| s.auto_pan_depth.set_value(0.0)),
            ),
            (
                "stopping the orbit",
                click_ratio(orbit, |s| s.auto_pan_rate.set_value(0.0)),
            ),
            (
                "turning the depth down",
                click_ratio(orbit, |s| s.auto_pan_depth.set_value(0.6)),
            ),
            (
                "switching the orbit on",
                click_ratio(
                    |_| {},
                    |s| {
                        s.auto_pan_depth.set_value(0.9);
                        s.auto_pan_rate.set_value(0.1);
                    },
                ),
            ),
            (
                "re-centring the balance",
                click_ratio(|p| p.pan.set_value(0.8), |s| s.pan.set_value(0.0)),
            ),
            (
                "moving the balance",
                click_ratio(|p| p.pan.set_value(0.3), |s| s.pan.set_value(0.5)),
            ),
        ];
        for (name, ratio) in cases {
            assert!(
                ratio < 4.0,
                "{name}: {ratio:.1}x rougher across the change than either side of it"
            );
        }
    }

    #[test]
    fn switching_on_opens_no_path_onto_silence() {
        // Coming out of rest every delay line is empty, and a path that reads
        // further back than has been written since reads silence until the
        // signal reaches it, then starts with a hard edge. The crossfeed reads
        // a whole head-width back from its very first sample. A steady input
        // makes any such edge stand out, because nothing else in it moves.
        //
        // Not quite nothing: the room fills from silence, so each reflection
        // arrives as a ramp up from zero, and the start of a ramp is a corner.
        // Those measure around 7e-5. The crossfeed's edge, when it was there,
        // measured over six times that.
        let mut s = spatializer(|_| {});
        let mut out = [0.0; 2];
        let mut tick = |s: &mut StereoSpatializer| {
            s.tick(&[0.5, 0.5], &mut out);
            (out[0], out[1])
        };
        let mut trace: Vec<(f32, f32)> = (0..4_800).map(|_| tick(&mut s)).collect();
        s.auto_pan_depth.set_value(0.9);
        trace.extend((0..4_800).map(|_| tick(&mut s)));
        let worst = roughness(&trace, 4_000..trace.len());
        assert!(
            worst < 2e-4,
            "switching on put a {worst:.1e} edge into a steady signal"
        );
    }

    #[test]
    fn re_engaging_does_not_replay_stale_audio() {
        // Every line here is written only while the effect is engaged. If one is
        // left holding whatever was playing when the effect was switched off,
        // switching it back on reads that back out — up to 50 ms of audio from
        // an entirely different part of the track, arriving as a burst out of
        // nowhere.
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(1.0);
            p.auto_pan_rate.set_value(0.0);
        });
        let mut out = [0.0; 2];
        let loud = |i: usize| (i as f32 / 48_000.0 * 300.0 * TAU).sin() * 0.9;

        // A loud passage with the effect on, filling every line.
        for i in 0..4_000 {
            s.tick(&[loud(i), loud(i)], &mut out);
        }

        // Switched off while the music carries on: the effect fades out and
        // drops into bypass with the loud passage still in its lines.
        s.auto_pan_depth.set_value(0.0);
        for i in 4_000..20_000 {
            s.tick(&[loud(i), loud(i)], &mut out);
        }
        assert!(
            s.at_rest,
            "a third of a second is ample to fade out and drop into bypass"
        );
        s.tick(&[0.3, -0.6], &mut out);
        assert_eq!(out, [0.3, -0.6], "at rest, the bypass is bit-transparent");

        // The track goes quiet, still bypassed, and the effect comes back on.
        for _ in 0..2_000 {
            s.tick(&[0.0, 0.0], &mut out);
        }
        s.auto_pan_depth.set_value(1.0);
        let mut worst = 0.0f32;
        for _ in 0..4_000 {
            s.tick(&[0.0, 0.0], &mut out);
            worst = worst.max(out[0].abs()).max(out[1].abs());
        }
        assert!(
            worst < 1e-4,
            "re-engaging replayed {worst:.4} of stale audio into silence"
        );
    }

    #[test]
    fn silence_never_leaves_a_filter_in_subnormal_floats() {
        // Subnormal arithmetic runs in microcode. Engaged on digital silence —
        // a pause, the gap between tracks — this unit measured four times its
        // cost on music, on the thread where a stall is a dropout. This is the
        // deterministic stand-in for a timing test: once the music stops and
        // the filters have decayed, every state has to be exactly zero or a
        // normal float.
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(1.0);
            p.auto_pan_rate.set_value(0.35);
            p.pan.set_value(0.2);
        });
        let mut out = [0.0; 2];
        for i in 0..24_000 {
            let v = (i as f32 / 48_000.0 * 220.0 * TAU).sin() * 0.5;
            s.tick(&[v, v * 0.8], &mut out);
        }
        for _ in 0..96_000 {
            s.tick(&[0.0, 0.0], &mut out);
        }

        let subnormal = |x: &f32| *x != 0.0 && !x.is_normal();
        let states = [
            s.shadow_l,
            s.shadow_r,
            s.own_low_l,
            s.own_low_r,
            s.ild_l,
            s.ild_r,
            s.er_damp_l,
            s.er_damp_r,
            s.pinna_l[0],
            s.pinna_l[1],
            s.pinna_r[0],
            s.pinna_r[1],
        ];
        assert!(
            !states.iter().any(subnormal),
            "filter states left subnormal: {states:?}"
        );
        for line in [&s.delay_l, &s.delay_r, &s.er_l, &s.er_r] {
            assert!(
                !line.iter().any(subnormal),
                "a delay line is holding subnormal samples"
            );
        }
    }

    /// Not a pass/fail test — a measuring tool, kept because this crate's
    /// history is a bug report about audio dropping out under CPU pressure,
    /// and every cue added here costs per-sample work in the callback.
    ///
    /// Measured on the machine this was written on: 0.05% of one core
    /// bypassed, 0.64% fully engaged, and 0.68% engaged on digital silence —
    /// which, before `flush_denormal`, was 3.0%. Run with
    /// `cargo test --release -p melotui-audio -- --ignored --nocapture`.
    #[test]
    #[ignore = "measurement, not a regression guard; timing-dependent"]
    fn bench_cost() {
        use std::time::Instant;
        const SR: f32 = 48_000.0;
        const N: usize = 48_000 * 20; // 20 seconds of audio
        let tone = |i: usize| (i as f32 / SR * 220.0 * TAU).sin() * 0.3;
        for (name, depth, rate, level) in [
            ("bypass", 0.0, 0.0, 1.0),
            ("engaged", 1.0, 0.35, 1.0),
            ("silence", 1.0, 0.35, 0.0),
        ] {
            let mut s = spatializer(|p| {
                p.auto_pan_depth.set_value(depth);
                p.auto_pan_rate.set_value(rate);
            });
            let mut out = [0.0; 2];
            // Silence only costs anything once the filters have decayed into
            // it, so every run starts from music.
            for i in 0..48_000 {
                s.tick(&[tone(i), tone(i) * 0.8], &mut out);
            }
            let t = Instant::now();
            for i in 0..N {
                let v = tone(i) * level;
                s.tick(&[v, v * 0.8], &mut out);
            }
            let el = t.elapsed().as_secs_f64();
            println!(
                "{name:>8}: {:.3}s for 20s audio  =>  {:.4}% of one core (realtime budget)",
                el,
                el / 20.0 * 100.0
            );
            std::hint::black_box(out);
        }
    }
}
