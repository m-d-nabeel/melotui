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

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;
use std::f32::consts::TAU;

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
/// only ever reached its useful range at extreme settings.
const MIN_CROSSFEED_SCALE: f32 = 0.5;
/// How far a control has to travel for the crossfeed floor to be fully faded
/// in. Short enough that crossfeed is present for any real setting, long enough
/// that reaching it is a fade and not a step.
const CROSSFEED_RAMP_TRAVEL: f32 = 0.05;
/// How fast the pan position follows its target, per sample at 48 kHz.
const PAN_SMOOTHING_HZ: f32 = 12.0;

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
/// Half-width of the notch as a ratio either side of centre. Narrow enough
/// that the skirt does not reach down into the presence region — a notch that
/// bled into 3 kHz would dull vocals every time the image swung behind the
/// head, which is exactly the complaint the previous front/back cue earned.
const PINNA_Q_SPREAD: f32 = 1.45;
/// Ceiling on the band-pass normalisation.
///
/// The normalisation is the reciprocal of the band's peak gain, so a band that
/// came out near-zero — a degenerate spread, or a centre frequency crowding
/// Nyquist at a low sample rate — would otherwise ask for unbounded gain.
const MAX_PINNA_BAND_GAIN: f32 = 6.0;
/// Depth of the notch at full orbit depth, as a fraction of the band.
///
/// Real pinna notches are deeper than this. It is kept at 0.7 (about -10 dB)
/// because the centre frequency sweeps continuously with the orbit, and a
/// deeper moving notch stops reading as a direction and starts reading as a
/// wah pedal. Bounded below 1.0 regardless, so the subtraction can never invert
/// the band into a phase-flipped peak.
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
/// The two patterns share no delay time and no common divisor, so the beds are
/// decorrelated and add width instead of a phantom centre. Gains fall roughly
/// as 1/distance, as reflections off a small room's surfaces do.
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

#[derive(Clone)]
pub struct StereoSpatializer {
    pan: Shared,
    auto_pan_rate: Shared,
    auto_pan_depth: Shared,
    sample_rate: f32,

    phase: f32,
    smoothed_pan: f32,
    pan_coeff: f32,

    /// One delay line per channel: the whole point is that they stay separate.
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    write_idx: usize,

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
    /// Cascaded one-pole states forming the pinna notch's band-pass. Two poles
    /// per edge give the band 12 dB/octave skirts, which is enough for the
    /// subtraction to cut a real notch rather than a shallow dip.
    pinna_lo_l: [f32; 2],
    pinna_hi_l: [f32; 2],
    pinna_lo_r: [f32; 2],
    pinna_hi_r: [f32; 2],

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
    /// Whether the room line currently holds live audio, so that bypassing can
    /// clear it exactly once instead of on every sample.
    room_is_live: bool,
    /// Band-pass normalisation at the two extremes of the notch's travel.
    /// Computed when the sample rate is set rather than per sample: it costs
    /// transcendentals, and the orbit moves the centre frequency far too slowly
    /// for the difference to matter.
    pinna_gain_front: f32,
    pinna_gain_rear: f32,
}

impl StereoSpatializer {
    pub fn new(params: &AudioParams) -> Self {
        let mut s = Self {
            pan: params.pan.clone(),
            auto_pan_rate: params.auto_pan_rate.clone(),
            auto_pan_depth: params.auto_pan_depth.clone(),
            sample_rate: DEFAULT_SR as f32,
            phase: 0.0,
            smoothed_pan: 0.0,
            pan_coeff: 0.0,
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            write_idx: 0,
            shadow_l: 0.0,
            shadow_r: 0.0,
            own_low_l: 0.0,
            own_low_r: 0.0,
            ild_l: 0.0,
            ild_r: 0.0,
            pinna_lo_l: [0.0; 2],
            pinna_hi_l: [0.0; 2],
            pinna_lo_r: [0.0; 2],
            pinna_hi_r: [0.0; 2],
            er_l: Vec::new(),
            er_r: Vec::new(),
            er_idx: 0,
            er_damp_l: 0.0,
            er_damp_r: 0.0,
            er_offsets_l: [0; 5],
            er_offsets_r: [0; 5],
            room_is_live: false,
            pinna_gain_front: 1.0,
            pinna_gain_rear: 1.0,
        };
        s.reconfigure();
        s
    }

    fn reconfigure(&mut self) {
        let len = Ord::max((self.sample_rate * MAX_ITD_SECONDS).ceil() as usize + 4, 4);
        self.delay_l = vec![0.0; len];
        self.delay_r = vec![0.0; len];
        self.write_idx = 0;
        self.pan_coeff = 1.0 - (-TAU * PAN_SMOOTHING_HZ / self.sample_rate).exp();

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

        self.pinna_gain_front = Self::pinna_notch_scale(PINNA_FRONT_HZ, self.sample_rate);
        self.pinna_gain_rear = Self::pinna_notch_scale(PINNA_REAR_HZ, self.sample_rate);
    }

    /// Constant-power balance, so sweeping across centre does not dip.
    fn balance_gains(pan: f32) -> (f32, f32) {
        let angle = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * TAU * 0.5; // 0..π/2
        (angle.cos(), angle.sin())
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

    /// One-pole low-pass; the head's shadow on the far ear.
    fn shadow(state: &mut f32, sample: f32, cutoff_hz: f32, sample_rate: f32) -> f32 {
        let alpha = 1.0 - (-TAU * cutoff_hz / sample_rate).exp();
        *state += alpha * (sample - *state);
        *state
    }

    /// Two cascaded one-poles, for a 12 dB/octave edge.
    fn shadow2(state: &mut [f32; 2], sample: f32, cutoff_hz: f32, sample_rate: f32) -> f32 {
        let alpha = 1.0 - (-TAU * cutoff_hz / sample_rate).exp();
        state[0] += alpha * (sample - state[0]);
        state[1] += alpha * (state[0] - state[1]);
        state[1]
    }

    /// Splits `sample` at [`ILD_SPLIT_HZ`] and attenuates only the high band.
    ///
    /// The broadband balance law is left to apply on top of this, so the low
    /// band comes through exactly as it did before this stage existed and only
    /// the high band gains the extra head shadow.
    fn hf_shadow(state: &mut f32, sample: f32, keep_high: f32, sample_rate: f32) -> f32 {
        let low = Self::shadow(state, sample, ILD_SPLIT_HZ, sample_rate);
        low + (sample - low) * keep_high
    }

    /// The one-pole coefficient for `cutoff_hz`, and the pair of edge cutoffs
    /// the notch's band-pass is built from.
    ///
    /// Shared by the filter and by its normalisation so the two can never be
    /// calibrated against different frequencies.
    fn pinna_edges(centre_hz: f32, sample_rate: f32) -> (f32, f32) {
        // Nyquist guard: at low sample rates the upper edge could otherwise be
        // asked for a cutoff it cannot represent, where the one-pole's response
        // stops being a low-pass.
        let nyquist = sample_rate * 0.5;
        let upper = (centre_hz * PINNA_Q_SPREAD).min(nyquist * 0.9);
        let lower = (centre_hz / PINNA_Q_SPREAD).min(upper * 0.99);
        (lower, upper)
    }

    /// The scalar the band must be multiplied by for the subtraction to reach
    /// [`MAX_PINNA_DEPTH`] of attenuation at the centre frequency.
    ///
    /// Not simply the reciprocal of the band's peak gain, which is the obvious
    /// answer and the wrong one: the band-pass has a phase shift at its own
    /// centre, so `1 - k*H` is a *complex* subtraction. Scaling `H` to unit
    /// magnitude leaves `|1 - k*H|` anywhere between `1 - k` and `1 + k`
    /// depending on that phase, and the phase moves with sample rate — which is
    /// exactly how a notch calibrated by measurement at 48 kHz came out 3 dB
    /// shallower at 96 kHz.
    ///
    /// So solve for it instead. Writing `A` for the wanted residual and `H` for
    /// the band's complex response at the centre:
    ///
    /// ```text
    /// |1 - k*H|^2 = A^2   =>   k^2|H|^2 - 2k*Re(H) + 1 - A^2 = 0
    /// ```
    ///
    /// whose smaller positive root is the least gain that gets there.
    fn pinna_notch_scale(centre_hz: f32, sample_rate: f32) -> f32 {
        let (lower, upper) = Self::pinna_edges(centre_hz, sample_rate);
        let theta = TAU * centre_hz / sample_rate;
        let (sin_t, cos_t) = (-theta).sin_cos();

        // Two cascaded one-poles: H(z) = (a / (1 - (1-a) z^-1))^2, on the unit
        // circle at the centre frequency.
        let response = |cutoff: f32| -> (f32, f32) {
            let a = 1.0 - (-TAU * cutoff / sample_rate).exp();
            let (re, im) = (1.0 - (1.0 - a) * cos_t, -(1.0 - a) * sin_t);
            let den = re * re + im * im;
            if den < 1e-20 {
                return (0.0, 0.0);
            }
            let (r1, i1) = (a * re / den, -a * im / den);
            (r1 * r1 - i1 * i1, 2.0 * r1 * i1)
        };

        let (ur, ui) = response(upper);
        let (lr, li) = response(lower);
        let (hr, hi) = (ur - lr, ui - li);
        let mag_sq = hr * hr + hi * hi;
        if mag_sq < 1e-12 {
            return 0.0;
        }

        let residual = 1.0 - MAX_PINNA_DEPTH;
        let disc = hr * hr - mag_sq * (1.0 - residual * residual);
        let k = if disc < 0.0 {
            // The phase makes the wanted depth unreachable at any gain. Fall
            // back to the projection, which is the deepest notch this band can
            // cut — a shallower cue, never a boost.
            hr / mag_sq
        } else {
            (hr - disc.sqrt()) / mag_sq
        };
        k.clamp(0.0, MAX_PINNA_BAND_GAIN)
    }

    /// Cuts a notch centred on `centre_hz` by subtracting a band-pass of the
    /// input from it.
    ///
    /// Built from low-passes rather than a resonant biquad on purpose: the
    /// centre frequency is swept continuously by the orbit, and a resonant
    /// section re-tuned every sample can ring or go unstable. A difference of
    /// two cascaded one-poles cannot — it has no feedback path around the
    /// modulated coefficient — and `depth` is bounded below 1.0 so the
    /// subtraction cannot overshoot into a phase-inverted peak.
    fn pinna_notch(
        lo_state: &mut [f32; 2],
        hi_state: &mut [f32; 2],
        sample: f32,
        centre_hz: f32,
        scale: f32,
        sample_rate: f32,
    ) -> f32 {
        let (lower, upper) = Self::pinna_edges(centre_hz, sample_rate);
        let band = Self::shadow2(hi_state, sample, upper, sample_rate)
            - Self::shadow2(lo_state, sample, lower, sample_rate);
        sample - band * scale
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let rate = self.auto_pan_rate.value().clamp(0.0, 2.0);
        let depth = self.auto_pan_depth.value().clamp(0.0, 1.0);
        let static_pan = self.pan.value().clamp(-1.0, 1.0);

        let orbiting = depth > 0.001 && rate > 0.001;
        let auto = if orbiting {
            self.phase = (self.phase + rate / self.sample_rate).fract();
            (self.phase * TAU).sin() * depth
        } else {
            0.0
        };

        let target_pan = (static_pan + auto).clamp(-1.0, 1.0);
        self.smoothed_pan += (target_pan - self.smoothed_pan) * self.pan_coeff;
        let pan = self.smoothed_pan;
        let (gain_l, gain_r) = Self::balance_gains(pan);

        // Nothing asked for: stay bit-transparent rather than paying for a
        // spatialiser that is not doing anything.
        if !orbiting && static_pan.abs() < 0.001 && depth < 0.001 {
            // Empty the signal path on the way out. Every delay line here is
            // written only while the effect is engaged, so a line left full
            // during bypass is a recording of whatever was playing when it was
            // last switched on, waiting to be read straight back out the moment
            // it is switched on again — the interaural lines feed the crossfeed,
            // the crossfeed feeds the room, and the room holds it for a further
            // 50 ms, so leaving any one of them behind is audible. Clearing on
            // the transition rather than every sample keeps the bypass path
            // free.
            if self.room_is_live {
                self.clear_signal_state();
                self.room_is_live = false;
            }
            return (left, right);
        }
        self.room_is_live = true;

        self.delay_l[self.write_idx] = left;
        self.delay_r[self.write_idx] = right;

        // The ear further from the source hears it later.
        let itd = self.sample_rate * MAX_ITD_SECONDS;
        let delay_l = pan.max(0.0) * itd;
        let delay_r = (-pan).max(0.0) * itd;
        let direct_l = Self::read(&self.delay_l, self.write_idx, delay_l);
        let direct_r = Self::read(&self.delay_r, self.write_idx, delay_r);

        // Interaural level difference, the part the balance law cannot express.
        // The far ear loses its high band to the head; the near ear keeps
        // everything. At centre this is identity in both ears.
        let hf_shadow = pan.abs() * MAX_HF_SHADOW;
        let (direct_l, direct_r) = if hf_shadow > 0.001 {
            let (keep_l, keep_r) = if pan > 0.0 {
                (1.0 - hf_shadow, 1.0) // source right: the left ear is shadowed
            } else {
                (1.0, 1.0 - hf_shadow)
            };
            (
                Self::hf_shadow(&mut self.ild_l, direct_l, keep_l, self.sample_rate),
                Self::hf_shadow(&mut self.ild_r, direct_r, keep_r, self.sample_rate),
            )
        } else {
            (direct_l, direct_r)
        };

        // Crossfeed carries the *opposite* channel, delayed by the full head
        // width and shadowed. Because it is low-passed, separation above the
        // cutoff is untouched — which is what preserves the stereo image.
        let engagement = depth.max(static_pan.abs());
        // The floor is faded in across the first sliver of travel rather than
        // applied as a step. Without the ramp, nudging a control from 0.000 to
        // 0.002 would take the crossfeed from nothing to half its range between
        // one sample and the next, which is a discontinuity in the output and
        // therefore a click.
        let floor_ramp = (engagement / CROSSFEED_RAMP_TRAVEL).min(1.0);
        let crossfeed = MAX_CROSSFEED
            * (MIN_CROSSFEED_SCALE * floor_ramp + (1.0 - MIN_CROSSFEED_SCALE) * engagement);
        let (mut out_l, mut out_r) = if crossfeed > 0.001 {
            let cross_r = Self::shadow(
                &mut self.shadow_l,
                Self::read(&self.delay_r, self.write_idx, itd),
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            let cross_l = Self::shadow(
                &mut self.shadow_r,
                Self::read(&self.delay_l, self.write_idx, itd),
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            // A shuffler, not a blend: each ear gives up as much of its *own*
            // low band as it receives of the other's, so the crossfeed moves
            // bass toward the centre instead of adding it.
            //
            // The obvious alternative — mix the crossfeed in and divide by
            // `1 + crossfeed` — was what this did before, and it is wrong in a
            // way that is easy to miss and easy to hear. The crossfeed is
            // low-passed, so it only adds energy below the shadow cutoff, but
            // that normalisation is broadband: it paid for two octaves of added
            // bass by attenuating the entire spectrum, and engaging the
            // spatialiser measured 2.1 dB quieter than bypassing it. An effect
            // that turns the music down loses every A/B it is ever given,
            // however good the rest of it is.
            let own_low_l = Self::shadow(
                &mut self.own_low_l,
                direct_l,
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            let own_low_r = Self::shadow(
                &mut self.own_low_r,
                direct_r,
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            (
                direct_l + (cross_r - own_low_l) * crossfeed,
                direct_r + (cross_l - own_low_r) * crossfeed,
            )
        } else {
            (direct_l, direct_r)
        };

        self.write_idx = (self.write_idx + 1) % self.delay_l.len();

        // Front/back is cued by the pinna notch sliding down the spectrum as
        // the source passes behind the head. Applied per channel to its own
        // signal, so unlike a rear-centre cue it costs no separation.
        if orbiting {
            // 0 in front, 1 directly behind. `phase` is the orbit angle, so
            // this stays coherent with the ITD and the balance rather than
            // being an independent wobble.
            let behind = ((self.phase * TAU).cos() + 1.0) * 0.5;
            let centre = PINNA_FRONT_HZ + (PINNA_REAR_HZ - PINNA_FRONT_HZ) * behind;
            // The normalisation is smooth and monotonic between the two
            // endpoints, so tracking the centre with the same interpolation
            // keeps the notch the depth it claims all the way round the orbit.
            let solved =
                self.pinna_gain_front + (self.pinna_gain_rear - self.pinna_gain_front) * behind;
            // `solved` reaches full depth; the orbit depth scales the cue down
            // from there.
            let scale = solved * depth;
            if scale > 0.001 {
                out_l = Self::pinna_notch(
                    &mut self.pinna_lo_l,
                    &mut self.pinna_hi_l,
                    out_l,
                    centre,
                    scale,
                    self.sample_rate,
                );
                out_r = Self::pinna_notch(
                    &mut self.pinna_lo_r,
                    &mut self.pinna_hi_r,
                    out_r,
                    centre,
                    scale,
                    self.sample_rate,
                );
            }
        }

        let mut out_l = out_l * gain_l * std::f32::consts::SQRT_2;
        let mut out_r = out_r * gain_r * std::f32::consts::SQRT_2;

        // The room. Tied to orbit depth rather than to `engagement`, because a
        // listener nudging the balance control wants a balance control, not a
        // room; asking for the orbit is asking to be put somewhere.
        //
        // The bed is driven unconditionally and only *mixed* conditionally: a
        // line that is written only while it is audible is stale the instant it
        // becomes audible again, which is the same trap as bypassing without
        // clearing, one level down.
        let room = MAX_ROOM * depth;
        let (wet_l, wet_r) = self.early_reflections(out_l, out_r);
        if room > 0.001 {
            // Equal-power, using the bed's own energy sum: the taps are
            // decorrelated, so their contributions add in power, not amplitude.
            // Normalising by the amplitude sum instead would make the mix
            // quieter the more room was asked for.
            let norm = 1.0 / (1.0 + room * room * ER_ENERGY).sqrt();
            out_l = (out_l + wet_l * room) * norm;
            out_r = (out_r + wet_r * room) * norm;
        }

        (out_l, out_r)
    }

    /// Empties every delay line and filter state, leaving the orbit phase and
    /// the smoothed pan position alone.
    ///
    /// Those two are position, not signal: zeroing them would snap the image
    /// back to centre, which is a jump rather than the silence this is for.
    fn clear_signal_state(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.shadow_l = 0.0;
        self.shadow_r = 0.0;
        self.own_low_l = 0.0;
        self.own_low_r = 0.0;
        self.ild_l = 0.0;
        self.ild_r = 0.0;
        self.pinna_lo_l = [0.0; 2];
        self.pinna_hi_l = [0.0; 2];
        self.pinna_lo_r = [0.0; 2];
        self.pinna_hi_r = [0.0; 2];
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
            Self::shadow(&mut self.er_damp_l, wet_l, ER_DAMPING_HZ, self.sample_rate),
            Self::shadow(&mut self.er_damp_r, wet_r, ER_DAMPING_HZ, self.sample_rate),
        )
    }
}

impl AudioUnit for StereoSpatializer {
    fn reset(&mut self) {
        self.phase = 0.0;
        self.smoothed_pan = self.pan.value().clamp(-1.0, 1.0);
        self.clear_signal_state();
        self.room_is_live = false;
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

    pub(super) fn spatializer(setup: impl Fn(&AudioParams)) -> StereoSpatializer {
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

    /// Gain of the notch alone at `f`, in dB, with the notch centred on
    /// `centre`.
    fn notch_db(centre: f32, f: f32, sample_rate: f32) -> f32 {
        let (mut lo, mut hi) = ([0.0; 2], [0.0; 2]);
        let n = Ord::max(sample_rate as usize, 4_000) / 2;
        let out: Vec<f32> = (0..n)
            .map(|i| {
                let x = (i as f32 / sample_rate * f * TAU).sin();
                StereoSpatializer::pinna_notch(
                    &mut lo,
                    &mut hi,
                    x,
                    centre,
                    StereoSpatializer::pinna_notch_scale(centre, sample_rate),
                    sample_rate,
                )
            })
            .collect();
        let s = &out[n / 3..];
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &v) in s.iter().enumerate() {
            let ph = i as f64 / sample_rate as f64 * f as f64 * std::f64::consts::TAU;
            re += v as f64 * ph.cos();
            im += v as f64 * ph.sin();
        }
        20.0 * ((2.0 * (re * re + im * im).sqrt() / s.len() as f64) as f32)
            .max(1e-12)
            .log10()
    }

    #[test]
    fn the_notch_removes_the_fraction_of_the_band_it_says_it_does() {
        // `depth` is meant to mean "this fraction of the band is removed", so
        // depth 0.7 must measure as roughly -10 dB at the centre. It only does
        // if PINNA_BAND_GAIN matches the band-pass's actual peak; get that
        // wrong and every other constant here is being tuned against a notch
        // that is quietly a different depth than it claims.
        let expected = 20.0 * (1.0 - MAX_PINNA_DEPTH).log10();
        for sr in [44_100.0f32, 48_000.0, 96_000.0] {
            for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
                let got = notch_db(centre, centre, sr);
                assert!(
                    (got - expected).abs() < 2.0,
                    "sr {sr}, centre {centre}: notch is {got:.1} dB, expected about {expected:.1} dB"
                );
            }
        }
    }

    #[test]
    fn the_notch_leaves_the_midrange_alone() {
        // The whole reason the previous front/back cue was unusable: it dulled
        // vocals and guitars every time the image swung behind the head. A cue
        // the listener hears as "the tone changed" is not a direction cue.
        for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
            for f in [200.0f32, 500.0, 1_000.0] {
                let g = notch_db(centre, f, 48_000.0);
                assert!(
                    g.abs() < 1.0,
                    "centre {centre}: {g:.2} dB at {f} Hz — the midrange is being touched"
                );
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
        let mut best: f32 = 0.0;
        for f in [3_500.0f32, 5_000.0, 6_500.0, 9_000.0] {
            let front = notch_db(PINNA_FRONT_HZ, f, 48_000.0);
            let rear = notch_db(PINNA_REAR_HZ, f, 48_000.0);
            best = best.max((front - rear).abs());
        }
        assert!(
            best > 3.0,
            "front and back differ by at most {best:.1} dB — too little to cue anything"
        );
    }

    #[test]
    fn the_notch_never_boosts() {
        // The notch is a subtraction, so an over-large depth or a mis-set band
        // gain would not merely be wrong — it would flip the band's phase and
        // turn the notch into a peak. Nothing anywhere may come out louder.
        for centre in [PINNA_FRONT_HZ, PINNA_REAR_HZ] {
            for f in [
                100.0f32, 400.0, 1_500.0, 3_000.0, 6_000.0, 9_000.0, 15_000.0,
            ] {
                let g = notch_db(centre, f, 48_000.0);
                assert!(g < 0.5, "centre {centre}: {f} Hz is boosted by {g:.2} dB");
            }
        }
    }

    // ── The room ──────────────────────────────────────────

    /// Impulse response of the spatialiser to a left-only impulse, as
    /// (left, right) energy in the direct window and in the reflection window.
    ///
    /// The windows are chosen from the tap layout: every reflection lands after
    /// 11 ms, and everything before 5 ms is direct sound plus the crossfeed's
    /// short low-passed tail.
    fn impulse_windows(depth: f32, pan: f32) -> ((f64, f64), (f64, f64)) {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(depth);
            p.auto_pan_rate.set_value(0.0);
            p.pan.set_value(pan);
        });
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let mut out = [0.0; 2];
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
    /// Not a pass/fail test — a measuring tool, kept because this crate's
    /// history is a bug report about audio dropping out under CPU pressure,
    /// and every cue added here costs per-sample work in the callback.
    ///
    /// Measured on the machine this was written on: 0.13% of one core
    /// bypassed, 0.77% fully engaged. Run with
    /// `cargo test --release -p melotui-audio -- --ignored --nocapture`.
    #[test]
    #[ignore = "measurement, not a regression guard; timing-dependent"]
    fn bench_cost() {
        use std::time::Instant;
        const SR: f32 = 48_000.0;
        const N: usize = 48_000 * 20; // 20 seconds of audio
        for (name, depth, rate) in [("bypass", 0.0, 0.0), ("engaged", 1.0, 0.35)] {
            let mut s = spatializer(|p| {
                p.auto_pan_depth.set_value(depth);
                p.auto_pan_rate.set_value(rate);
            });
            let mut out = [0.0; 2];
            let t = Instant::now();
            for i in 0..N {
                let v = (i as f32 / SR * 220.0 * TAU).sin() * 0.3;
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

    #[test]
    fn re_engaging_the_room_does_not_replay_stale_audio() {
        // The room line is only written while the room is mixed in. If it is
        // left holding whatever was playing the last time the effect was on,
        // then turning the orbit back up reads that back — up to 50 ms of
        // audio from an entirely different part of the track, arriving as a
        // burst out of nowhere.
        let mut s = spatializer(|_| {});
        let params = AudioParams::new();
        let _ = &params;
        let mut out = [0.0; 2];

        // Loud passage with the effect engaged, filling the room line.
        s.pan.set_value(0.0);
        s.auto_pan_depth.set_value(1.0);
        s.auto_pan_rate.set_value(0.0);
        for i in 0..4_000 {
            let v = (i as f32 / 48_000.0 * 300.0 * TAU).sin() * 0.9;
            s.tick(&[v, v], &mut out);
        }

        // Effect off, and the track goes silent.
        s.auto_pan_depth.set_value(0.0);
        for _ in 0..2_000 {
            s.tick(&[0.0, 0.0], &mut out);
        }

        // Effect back on, still silent. Nothing may come out.
        s.auto_pan_depth.set_value(1.0);
        let mut worst = 0.0f32;
        for _ in 0..4_000 {
            s.tick(&[0.0, 0.0], &mut out);
            worst = worst.max(out[0].abs()).max(out[1].abs());
        }
        assert!(
            worst < 1e-4,
            "re-engaging the room replayed {worst:.4} of stale audio into silence"
        );
    }
}
