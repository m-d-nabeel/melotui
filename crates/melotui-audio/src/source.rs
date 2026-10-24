use fundsp::buffer::BufferVec;
use fundsp::hacker32::*;
use rodio::Source;
use rodio::source::SeekError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How long a switch into or out of the bypass takes. Long enough that the
/// changeover is a fade rather than a step, short enough that it still feels
/// like a switch.
const BYPASS_FADE_SECONDS: f32 = 0.010;

/// Holds the unprocessed signal back by the graph's latency, so that dry and
/// processed line up sample for sample.
///
/// The graph runs late — the limiter's lookahead alone is 5 ms — and a bypass
/// that skipped the delay would jump the music forward by that much every time
/// it engaged, and back every time it let go.
struct DryLine {
    frames: Vec<[f32; 2]>,
    idx: usize,
}

impl DryLine {
    fn new(latency: usize) -> Self {
        Self {
            frames: vec![[0.0; 2]; latency + 1],
            idx: 0,
        }
    }

    /// Stores `frame` and returns the one from `latency` frames ago.
    fn push(&mut self, frame: [f32; 2]) -> [f32; 2] {
        self.frames[self.idx] = frame;
        self.idx = (self.idx + 1) % self.frames.len();
        self.frames[self.idx]
    }

    fn clear(&mut self) {
        self.frames.fill([0.0; 2]);
        self.idx = 0;
    }
}

/// FunDSP -> Rodio adapter
/// block size 64
pub struct FundspSource<S> {
    source: S,
    unit: Box<dyn AudioUnit>,
    input_channels: usize,

    // block processing
    input_buffer: BufferVec,
    output_buffer: BufferVec,

    // output queue
    output_queue: Vec<f32>,
    current_idx: usize,
    bypass: Arc<AtomicBool>,
    /// How much of the processed signal is in the output: 0 bypassed, 1 fully
    /// processed. It fades toward wherever `bypass` points rather than jumping,
    /// because processed and dry can be very different signals — panned,
    /// delayed, carrying a reverb tail — and swapping one for the other between
    /// two samples is a click.
    wet: f32,
    /// Whether the graph has been running since it was last reset, i.e.
    /// whether its state follows the music rather than some earlier moment.
    graph_live: bool,
    /// The sample rate the graph is configured for.
    sample_rate: u32,
    /// The graph's latency, in frames.
    latency: usize,
    /// Frames still to go after a reset before the graph's output means
    /// anything; the fade toward it waits until then.
    priming: usize,
    dry: DryLine,
}

impl<S> FundspSource<S>
where
    S: Source<Item = f32>,
{
    const OUTPUT_CHANNELS: usize = 2;

    pub fn new(source: S, unit: Box<dyn AudioUnit>, bypass: Arc<AtomicBool>) -> Self {
        let input_channels = source.channels() as usize;
        let mut unit = unit;
        let block_size = 64;

        let sample_rate = source.sample_rate();
        unit.reset();
        unit.set_sample_rate(sample_rate as f64);
        let latency = Self::latency_of(&mut unit);

        let input_buffer = BufferVec::new(input_channels);
        let output_buffer = BufferVec::new(Self::OUTPUT_CHANNELS); // Always output stereo
        // A new track starts wherever the switch already is; there is no
        // earlier output to fade from.
        let bypassed = bypass.load(Ordering::Relaxed);

        Self {
            source,
            unit,
            input_channels,
            input_buffer,
            output_buffer,
            output_queue: Vec::with_capacity(block_size * Self::OUTPUT_CHANNELS),
            current_idx: 0,
            bypass,
            wet: if bypassed { 0.0 } else { 1.0 },
            graph_live: !bypassed,
            sample_rate,
            latency,
            priming: 0,
            dry: DryLine::new(latency),
        }
    }

    /// Retunes the graph for a new sample rate, and the dry side with it.
    ///
    /// This reallocates, so it is kept for genuine rate changes: a seek within
    /// a stream, which runs on the audio thread, only resets.
    fn configure(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.unit.reset();
        self.unit.set_sample_rate(sample_rate as f64);
        self.latency = Self::latency_of(&mut self.unit);
        self.dry = DryLine::new(self.latency);
        if self.graph_live {
            self.priming = self.latency;
        }
    }

    /// How many frames late the graph's output runs behind its input.
    fn latency_of(unit: &mut Box<dyn AudioUnit>) -> usize {
        unit.latency()
            .map_or(0, |frames| frames.round().max(0.0) as usize)
    }

    /// The unprocessed frame: mono goes to both sides, and anything wider than
    /// stereo keeps its first two channels.
    fn dry_frame(input: &BufferRef, input_channels: usize, i: usize) -> [f32; 2] {
        std::array::from_fn(|ch| {
            let ch_idx = if ch < input_channels { ch } else { 0 };
            input.channel_f32(ch_idx)[i]
        })
    }

    fn refill_buffer(&mut self) -> bool {
        self.output_queue.clear();
        self.current_idx = 0;

        let mut frames_read = 0;
        let block_size = 64;

        let mut input_view = self.input_buffer.buffer_mut();

        for i in 0..block_size {
            let mut frame_complete = true;
            for ch in 0..self.input_channels {
                if let Some(sample) = self.source.next() {
                    input_view.channel_f32_mut(ch)[i] = sample;
                } else {
                    frame_complete = false;
                    break;
                }
            }

            if frame_complete {
                frames_read += 1;
            } else {
                break;
            }
        }

        if frames_read == 0 {
            return false;
        }

        // A stream can change rate between spans; the graph has to follow,
        // or every filter and delay in it is tuned for the wrong one.
        let sample_rate = self.source.sample_rate();
        if sample_rate != self.sample_rate {
            self.configure(sample_rate);
        }

        let want_wet = !self.bypass.load(Ordering::Relaxed);
        let input_ref = self.input_buffer.buffer_ref();

        if !want_wet && self.wet == 0.0 {
            self.graph_live = false;
            for i in 0..frames_read {
                let dry = self
                    .dry
                    .push(Self::dry_frame(&input_ref, self.input_channels, i));
                self.output_queue.extend_from_slice(&dry);
            }
            return true;
        }

        if !self.graph_live {
            // Coming out of bypass. The graph has not run since it went in, so
            // every delay line in it — reverb, echo, the spatialiser's room —
            // still holds whatever was playing at that moment: audio from a
            // different point in the track, which would otherwise be the first
            // thing out of each of them.
            self.unit.reset();
            // And nothing it puts out means anything until its latency has
            // passed: the limiter's lookahead starts empty, so for those first
            // milliseconds the processed side is silence rather than music,
            // and fading toward it then would fade toward a gap.
            self.priming = self.latency;
            self.graph_live = true;
        }

        let mut output_mut = self.output_buffer.buffer_mut();

        self.unit.process(frames_read, &input_ref, &mut output_mut);

        let output_ref = self.output_buffer.buffer_ref();
        let step = 1.0 / (BYPASS_FADE_SECONDS * self.source.sample_rate() as f32).max(1.0);

        for i in 0..frames_read {
            let dry = self
                .dry
                .push(Self::dry_frame(&input_ref, self.input_channels, i));
            if self.priming > 0 {
                self.priming -= 1;
            } else if want_wet {
                self.wet = (self.wet + step).min(1.0);
            } else {
                self.wet = (self.wet - step).max(0.0);
            }
            // Smoothstep rather than a straight ramp, so the mix leaves and
            // arrives with zero slope instead of bending sharply at each end
            // of the fade. It also keeps the processed side almost silent for
            // the first few samples after a reset, which is when anything in
            // the graph starting from empty state settles.
            let mix = self.wet * self.wet * (3.0 - 2.0 * self.wet);
            for (ch, dry) in dry.into_iter().enumerate() {
                let wet = output_ref.channel_f32(ch)[i];
                let sample = if self.wet >= 1.0 {
                    wet
                } else {
                    dry + (wet - dry) * mix
                };
                self.output_queue.push(sample);
            }
        }

        true
    }
}

impl<S> Iterator for FundspSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.current_idx < self.output_queue.len() {
            let sample = self.output_queue[self.current_idx];
            self.current_idx += 1;
            return Some(sample);
        }

        if self.refill_buffer() {
            let sample = self.output_queue[self.current_idx];
            self.current_idx += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl<S> Source for FundspSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        // Spans are counted in samples, not frames. A mono input yields two
        // output samples for every one it consumes, so forwarding the inner
        // span verbatim under-reports it by half and makes rodio re-bootstrap
        // its resampler twice as often as the format actually requires.
        self.source.current_span_len().map(|samples| {
            samples
                .saturating_mul(Self::OUTPUT_CHANNELS)
                .saturating_div(Ord::max(self.input_channels, 1))
        })
    }

    fn channels(&self) -> u16 {
        // The DSP graph always outputs stereo even for mono sources.
        Self::OUTPUT_CHANNELS as u16
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.source.try_seek(pos)?;

        // flush buffers
        self.output_queue.clear();
        self.current_idx = 0;

        // reset dsp. The dry side starts over from silence too, so that it
        // stays in step with a graph that has just been emptied. With the rate
        // unchanged there is nothing to reallocate, and a seek runs on the
        // audio thread, so it does not.
        let sample_rate = self.source.sample_rate();
        if sample_rate == self.sample_rate {
            self.unit.reset();
            self.dry.clear();
            if self.graph_live {
                self.priming = self.latency;
            }
        } else {
            self.configure(sample_rate);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FundspSource, Source};
    use fundsp::hacker32::{AudioUnit, pass};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// Minimal source reporting spans the way symphonia does: in samples,
    /// i.e. frames * channels.
    struct Tone {
        channels: u16,
        span_samples: usize,
        remaining: usize,
    }

    impl Iterator for Tone {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.remaining == 0 {
                return None;
            }
            self.remaining -= 1;
            Some(0.25)
        }
    }

    impl Source for Tone {
        fn current_span_len(&self) -> Option<usize> {
            Some(self.span_samples)
        }
        fn channels(&self) -> u16 {
            self.channels
        }
        fn sample_rate(&self) -> u32 {
            48_000
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    fn unit() -> Box<dyn AudioUnit> {
        Box::new(pass() | pass())
    }

    #[test]
    fn span_len_is_reported_in_output_samples_for_mono() {
        // 1152 mono samples in == 1152 frames == 2304 stereo samples out.
        let src = Tone {
            channels: 1,
            span_samples: 1152,
            remaining: 4096,
        };
        let fs = FundspSource::new(src, unit(), Arc::new(AtomicBool::new(true)));
        assert_eq!(fs.channels(), 2);
        assert_eq!(fs.current_span_len(), Some(2304));
    }

    #[test]
    fn span_len_is_unchanged_for_stereo() {
        // Stereo preserves the sample count, so the span passes through as-is.
        let src = Tone {
            channels: 2,
            span_samples: 2304,
            remaining: 4096,
        };
        let fs = FundspSource::new(src, unit(), Arc::new(AtomicBool::new(true)));
        assert_eq!(fs.current_span_len(), Some(2304));
    }

    #[test]
    fn mono_input_really_does_emit_two_samples_per_input_sample() {
        let src = Tone {
            channels: 1,
            span_samples: 1152,
            remaining: 100,
        };
        let fs = FundspSource::new(src, unit(), Arc::new(AtomicBool::new(true)));
        assert_eq!(fs.count(), 200, "mono is upmixed to stereo");
    }

    /// Stereo source playing back a fixed script of frames.
    struct Script(std::vec::IntoIter<f32>);

    impl Script {
        fn new(frames: impl IntoIterator<Item = f32>) -> Self {
            Self(
                frames
                    .into_iter()
                    .flat_map(|v| [v, v])
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
        }
    }

    impl Iterator for Script {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            self.0.next()
        }
    }

    impl Source for Script {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            48_000
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    /// Pulls `frames` stereo frames.
    fn take<S: Source<Item = f32>>(fs: &mut FundspSource<S>, frames: usize) -> Vec<(f32, f32)> {
        (0..frames)
            .map(|_| {
                let l = fs.next().expect("source ran dry");
                let r = fs.next().expect("source ran dry");
                (l, r)
            })
            .collect()
    }

    /// Pulls `frames` stereo frames, returning the left channel.
    fn take_left<S: Source<Item = f32>>(fs: &mut FundspSource<S>, frames: usize) -> Vec<f32> {
        take(fs, frames).into_iter().map(|(l, _)| l).collect()
    }

    #[test]
    fn switching_the_effects_on_or_off_is_a_fade_not_a_step() {
        use std::sync::atomic::Ordering;

        // The worst case for a switch: a graph whose output is the input
        // upside down, so processed and dry are as far apart as two signals
        // can be. Swapping one for the other in a single sample is a step of
        // twice the signal, which is a click however brief.
        let bypass = Arc::new(AtomicBool::new(true));
        let mut fs = FundspSource::new(
            Script::new(std::iter::repeat_n(0.25, 48_000)),
            Box::new((pass() * -1.0) | (pass() * -1.0)),
            bypass.clone(),
        );

        let mut out = take_left(&mut fs, 4_800);
        bypass.store(false, Ordering::Relaxed);
        out.extend(take_left(&mut fs, 4_800));
        bypass.store(true, Ordering::Relaxed);
        out.extend(take_left(&mut fs, 4_800));

        assert!((out[4_799] - 0.25).abs() < 1e-6, "bypassed: dry");
        assert!((out[9_599] + 0.25).abs() < 1e-6, "engaged: processed");
        assert!((out[14_399] - 0.25).abs() < 1e-6, "bypassed again: dry");
        let worst = out
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(
            worst < 0.01,
            "the output jumped by {worst:.3} in one sample at a switch"
        );
    }

    #[test]
    fn effects_come_back_without_replaying_what_they_held() {
        use fundsp::hacker32::delay;
        use std::sync::atomic::Ordering;

        // Bypassing stops the graph, so every delay line in it — reverb,
        // echo, the spatialiser's room — stays holding whatever was playing at
        // that moment. Switching back on must not play that out: it is audio
        // from a different point in the track.
        let bypass = Arc::new(AtomicBool::new(false));
        let loud = std::iter::repeat_n(0.5, 9_600);
        let quiet = std::iter::repeat_n(0.0, 9_600);
        let mut fs = FundspSource::new(
            Script::new(loud.chain(quiet)),
            Box::new(delay(0.01) | delay(0.01)),
            bypass.clone(),
        );

        // Loud with the effects on, filling the delay line, then switched off
        // while it is still loud.
        take_left(&mut fs, 4_800);
        bypass.store(true, Ordering::Relaxed);
        take_left(&mut fs, 4_800);

        // The track goes quiet, still bypassed, and the effects come back on.
        take_left(&mut fs, 4_800);
        bypass.store(false, Ordering::Relaxed);
        let worst = take_left(&mut fs, 4_800)
            .into_iter()
            .fold(0.0f32, |a, v| a.max(v.abs()));
        assert!(
            worst < 1e-6,
            "switching back on replayed {worst:.3} of audio held from before the bypass"
        );
    }

    #[test]
    fn the_spatialiser_switches_on_and_off_cleanly_through_the_real_graph() {
        use crate::effects::{AudioParams, create_audio_graph};
        use std::sync::atomic::Ordering;

        // End to end, the way the worker drives it. With every other control
        // at its default, turning the orbit on takes the whole graph out of
        // bypass and turning it off puts it back, so the switch, the graph
        // reset and the spatialiser's own glide all land at once.
        const SR: usize = 48_000;
        // Block boundaries a quarter-cycle from a zero crossing of the tone,
        // so whatever happens at a switch happens with the signal near a peak.
        const ON: usize = 24_064;
        const OFF: usize = 72_064;

        let tone = |i: usize| (i as f32 / SR as f32 * 150.0 * std::f32::consts::TAU).sin() * 0.5;
        let params = AudioParams::new();
        let bypass = Arc::new(AtomicBool::new(params.is_all_default()));
        let mut fs = FundspSource::new(
            Script::new((0..2 * SR).map(tone)),
            create_audio_graph(2, &params),
            bypass.clone(),
        );
        let set = |depth: f32, rate: f32| {
            params.auto_pan_depth.set_value(depth);
            params.auto_pan_rate.set_value(rate);
            bypass.store(params.is_all_default(), Ordering::Relaxed);
        };
        // The limiter's 5 ms lookahead, which the dry side is held back by.
        let latency = fs.latency;
        assert_eq!(latency, 240, "the graph reports the limiter's lookahead");

        let mut out = take(&mut fs, ON);
        set(0.9, 0.1);
        out.extend(take(&mut fs, OFF - ON));
        set(0.0, 0.0);
        out.extend(take(&mut fs, 2 * SR - OFF));

        // Largest second difference in either channel: a click is a spike in
        // it that a smooth signal does not have.
        let roughness = |range: std::ops::Range<usize>| {
            let d2 = |a: f32, b: f32, c: f32| (c - 2.0 * b + a).abs();
            out[range]
                .windows(3)
                .map(|w| d2(w[0].0, w[1].0, w[2].0).max(d2(w[0].1, w[1].1, w[2].1)))
                .fold(0.0, f32::max)
        };
        let dry = roughness(SR / 4..ON).max(roughness(7 * SR / 4..2 * SR));
        let engaged = roughness(ON + SR / 3..OFF);
        let steady = dry.max(engaged);
        for (name, range) in [
            ("switching on", ON - 2..ON + SR / 3),
            ("switching off", OFF - 2..OFF + SR / 3),
        ] {
            let ratio = roughness(range) / steady;
            assert!(
                ratio < 4.0,
                "{name}: {ratio:.1}x rougher than the steady signal either side"
            );
        }
        assert!(
            (out[2 * SR - 1].0 - tone(2 * SR - 1 - latency)).abs() < 1e-6,
            "switched off, the output is the dry signal again"
        );
    }

    #[test]
    fn the_bypass_runs_exactly_as_late_as_the_graph() {
        use crate::effects::limiter::LinkedLimiter;
        use std::sync::atomic::Ordering;

        // A graph that does nothing but run late: the limiter alone, fed a
        // signal too quiet for it to touch, is its 5 ms lookahead and nothing
        // else. Through it, switching in and out of bypass should be
        // inaudible — the output is the input delayed by the graph's latency,
        // sample for sample, before, during and after each switch. A bypass
        // that skipped the delay would jump the music by that much at every
        // switch, and a fade that started before a freshly reset graph had
        // filled would fade toward silence.
        const SR: usize = 48_000;
        let tone = |i: usize| (i as f32 / SR as f32 * 440.0 * std::f32::consts::TAU).sin() * 0.5;
        let bypass = Arc::new(AtomicBool::new(true));
        let mut fs = FundspSource::new(
            Script::new((0..SR).map(tone)),
            Box::new(LinkedLimiter::new(SR as f32)),
            bypass.clone(),
        );
        let latency = fs.latency;
        assert_eq!(latency, 240, "the lookahead is reported as latency");

        let mut out = take_left(&mut fs, 12_000);
        bypass.store(false, Ordering::Relaxed);
        out.extend(take_left(&mut fs, 12_000));
        bypass.store(true, Ordering::Relaxed);
        out.extend(take_left(&mut fs, 12_000));

        let worst = (latency..out.len())
            .map(|i| (out[i] - tone(i - latency)).abs())
            .fold(0.0, f32::max);
        assert!(
            worst < 1e-5,
            "the output strayed {worst:.4} from the input delayed by the graph's latency"
        );
    }

    /// Stereo source that can seek, and that switches sample rate partway
    /// through, the way a chained stream can.
    struct Tape {
        frames: Vec<f32>,
        frame: usize,
        channel: usize,
        rate_change_at: usize,
    }

    impl Iterator for Tape {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            let v = *self.frames.get(self.frame)?;
            self.channel += 1;
            if self.channel == 2 {
                self.channel = 0;
                self.frame += 1;
            }
            Some(v)
        }
    }

    impl Source for Tape {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            if self.frame < self.rate_change_at {
                48_000
            } else {
                44_100
            }
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
        fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
            self.frame = (pos.as_secs_f64() * 48_000.0) as usize;
            self.channel = 0;
            Ok(())
        }
    }

    #[test]
    fn a_seek_starts_the_graph_and_the_dry_side_over_from_silence() {
        use crate::effects::limiter::LinkedLimiter;

        // Loud for the first half second, silent after. Seek from the loud
        // part into the silent one, processed and then bypassed: nothing
        // from before the seek may come out after it.
        for bypassed in [false, true] {
            let frames = (0..96_000)
                .map(|i| if i < 24_000 { 0.5 } else { 0.0 })
                .collect();
            let tape = Tape {
                frames,
                frame: 0,
                channel: 0,
                rate_change_at: usize::MAX,
            };
            let bypass = Arc::new(AtomicBool::new(bypassed));
            let mut fs = FundspSource::new(tape, Box::new(LinkedLimiter::new(48_000.0)), bypass);
            take_left(&mut fs, 12_000);
            fs.try_seek(Duration::from_secs(1)).unwrap();
            let after = take_left(&mut fs, 4_800);
            let worst = after.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
            assert!(
                worst == 0.0,
                "bypassed {bypassed}: {worst} of pre-seek audio came out after the seek"
            );
            assert_eq!(fs.latency, 240, "a seek at the same rate retunes nothing");
        }
    }

    #[test]
    fn the_graph_follows_a_change_of_sample_rate_mid_stream() {
        use crate::effects::limiter::LinkedLimiter;

        let tape = Tape {
            frames: vec![0.25; 48_000],
            frame: 0,
            channel: 0,
            rate_change_at: 24_000,
        };
        let mut fs = FundspSource::new(
            tape,
            Box::new(LinkedLimiter::new(48_000.0)),
            Arc::new(AtomicBool::new(false)),
        );
        take_left(&mut fs, 12_000);
        assert_eq!((fs.sample_rate, fs.latency), (48_000, 240));
        take_left(&mut fs, 20_000);
        assert_eq!(
            (fs.sample_rate, fs.latency),
            (44_100, 220),
            "the graph is still tuned for the old rate"
        );
    }
}
