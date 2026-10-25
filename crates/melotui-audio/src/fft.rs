use ringbuf::traits::Consumer;
use spectrum_analyzer::{
    FrequencyLimit, samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window,
};
use std::{collections::VecDeque, time::Instant};

const NUM_BINS: usize = 512;
const MIN_FREQ: f32 = 30.0; // catch sub-bass
const MAX_FREQ: f32 = 18_000.0;

/// How hard the playhead is pulled back to its place behind the newest audio,
/// per second of error, and how fast its speed adapts to what is really
/// arriving. Together they settle in a few seconds without overshoot, and are
/// gentle enough that audio arriving in lumps does not show as wobble.
const PULL: f64 = 1.0;
const ADAPT: f64 = 0.25;
/// A pause longer than this between ticks (playback paused, the worker busy)
/// starts the pacing over instead of racing to catch up.
const RESTART_AFTER: f64 = 0.25;

/// Turns the tapped stream into spectra, one per tick.
///
/// The output device does not take audio a sample at a time: it pulls a
/// period at once, from 5 ms up to a sizeable fraction of a second, and the
/// tap sees it arrive in those lumps. Analysing whatever had arrived drew a
/// lump as a flurry of frames and then held still until the next one. So the
/// window is placed instead by a playhead that moves at the rate audio is
/// really being played, kept far enough behind the newest sample that the
/// next lump always lands before it is needed — which is also roughly when
/// the device actually plays it, so the picture keeps time with the sound.
pub struct SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    consumer: C,
    fft_size: usize,
    sample_rate: u32,

    /// Recent audio; `history[0]` is sample number `first`.
    history: VecDeque<f32>,
    first: u64,
    /// How many samples have arrived in all.
    received: u64,

    /// Where the analysis window ends, counted in samples received.
    playhead: f64,
    /// How fast the playhead moves, in samples per second.
    speed: f64,
    last_tick: Option<Instant>,

    /// The biggest lump audio arrives in, held and slowly forgotten.
    lump: f64,
    arrived_last_tick: usize,

    window: Vec<f32>,
}

impl<C> SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    pub fn new(consumer: C, fft_size: usize, sample_rate: u32) -> Self {
        Self {
            consumer,
            fft_size,
            sample_rate,
            history: VecDeque::with_capacity(fft_size * 4),
            first: 0,
            received: 0,
            playhead: 0.0,
            speed: sample_rate as f64,
            last_tick: None,
            lump: 0.0,
            arrived_last_tick: 0,
            window: Vec::with_capacity(fft_size * BASS_SCALE),
        }
    }

    pub fn into_consumer(self) -> C {
        self.consumer
    }

    /// Forgets the audio it holds and starts its pacing over, for after a
    /// seek: what was waiting is from before it.
    pub fn restart(&mut self) {
        while self.consumer.try_pop().is_some() {}
        self.history.clear();
        self.first = self.received;
        self.playhead = self.received as f64;
        self.last_tick = None;
        self.arrived_last_tick = 0;
    }

    pub fn next_spectrum(&mut self) -> Option<Vec<f32>> {
        self.next_spectrum_at(Instant::now())
    }

    fn next_spectrum_at(&mut self, now: Instant) -> Option<Vec<f32>> {
        let mut arrived = 0;
        while let Some(sample) = self.consumer.try_pop() {
            self.history.push_back(sample);
            arrived += 1;
        }
        self.received += arrived as u64;

        let elapsed = self
            .last_tick
            .map_or(f64::INFINITY, |last| now.duration_since(last).as_secs_f64());
        self.last_tick = Some(now);
        let restart = elapsed > RESTART_AFTER;

        // A lump can straddle two ticks, so measure two ticks' worth. What
        // piled up over a pause says nothing about the device.
        if !restart {
            let seen = (arrived + self.arrived_last_tick) as f64;
            if seen > self.lump {
                self.lump = seen;
            } else {
                self.lump -= (self.lump - seen) * 0.002;
            }
        }
        self.arrived_last_tick = arrived;

        let rate = self.sample_rate as f64;
        let lag = (self.lump * 1.5).max(rate * 0.01);
        let target = self.received as f64 - lag;
        let behind = target - self.playhead;

        let old_end = self.window_end();
        if restart || behind.abs() > (self.lump * 4.0).max(rate * 0.5) {
            self.playhead = target;
        } else {
            self.speed += behind * ADAPT * elapsed;
            self.speed = self.speed.clamp(rate * 0.25, rate * 4.0);
            self.playhead += (self.speed + behind * PULL) * elapsed;
        }
        // Never past the newest sample: with nothing new, wait for it.
        self.playhead = self.playhead.min(self.received as f64);

        // The bass's longer window is centred where the short one is, on
        // audio already here, so the bass is drawn in time with the rest
        // rather than half its length late.
        let (short_len, long) = (self.fft_size as u64, (self.fft_size * BASS_SCALE) as u64);
        let end = self.window_end();
        let long_end = (end + (long - short_len) / 2).min(self.received);
        let long_start = long_end.checked_sub(long)?;
        let keep_from = long_start.min(end.saturating_sub(short_len));
        if keep_from > self.first {
            let drop = ((keep_from - self.first) as usize).min(self.history.len());
            self.history.drain(..drop);
            self.first += drop as u64;
        }

        let short_start = end.checked_sub(short_len)?;
        if end == old_end || long_start < self.first || short_start < long_start {
            return None;
        }
        let offset = (long_start - self.first) as usize;
        self.window.clear();
        self.window
            .extend(self.history.range(offset..offset + long as usize));
        let short_from = (short_start - long_start) as usize;

        let short = self.spectrum_of(&self.window[short_from..short_from + self.fft_size])?;
        let bass = self.spectrum_of(&self.window)?;
        Some(self.fill_log_bins(&short, &bass, NUM_BINS))
    }

    /// The bins of `samples`' spectrum, at their exact frequencies: `to_map`
    /// keys them by whole hertz, which moved bins out of the narrow bass
    /// bands they belonged in.
    fn spectrum_of(&self, samples: &[f32]) -> Option<Vec<(f32, f32)>> {
        let spectrum = samples_fft_to_spectrum(
            &hann_window(samples),
            self.sample_rate,
            FrequencyLimit::Range(20.0, 22_050.0),
            Some(&divide_by_N),
        )
        .ok()?;
        Some(
            spectrum
                .data()
                .iter()
                .map(|(freq, value)| (freq.val(), value.val()))
                .collect(),
        )
    }

    /// The sample the analysis window ends at.
    fn window_end(&self) -> u64 {
        self.playhead.max(0.0) as u64
    }

    /// The bands, from the short window's spectrum and, in the bass, the
    /// long one's.
    fn fill_log_bins(
        &self,
        short: &[(f32, f32)],
        bass: &[(f32, f32)],
        bin_count: usize,
    ) -> Vec<f32> {
        let short_hz = self.sample_rate as f32 / self.fft_size as f32;
        let bass_hz = short_hz / BASS_SCALE as f32;
        let short = Bins::new(short, short_hz, 11.0);
        let bass = Bins::new(bass, bass_hz, 0.0);
        let (log_min, log_max) = (MIN_FREQ.ln(), MAX_FREQ.ln());
        let (split_low, split_high) = (BASS_SPLIT.0.ln(), BASS_SPLIT.1.ln());

        (0..bin_count)
            .map(|i| {
                let norm_pos = i as f32 / (bin_count as f32 - 1.0);
                let log_freq = log_min + norm_pos * (log_max - log_min);
                let freq = log_freq.exp();
                // The long window's bins in the bass, the short one's above,
                // blended between so the two meet without a seam.
                let from_bass =
                    ((split_high - log_freq) / (split_high - split_low)).clamp(0.0, 1.0);
                let value = if from_bass >= 1.0 {
                    bass.read(freq)
                } else if from_bass <= 0.0 {
                    short.read(freq)
                } else {
                    bass.read(freq) * from_bass + short.read(freq) * (1.0 - from_bass)
                };
                // Heights go as amplitude to the HEIGHT_CURVE power, the highs
                // lifted for being quieter.
                value.powf(HEIGHT_CURVE) * HEIGHT_GAIN * (1.0 + norm_pos)
            })
            .collect()
    }
}

/// The bass is analysed over a window this many times as long as the rest:
/// its notes lie close together in hertz, and the short window's bins, 11.7 Hz
/// apart at 48 kHz, could only draw them as steps or, read between, as smooth
/// domes. A longer window tells them apart and draws real peaks there, as cava
/// does, at the cost of the bass answering over a longer stretch of time.
const BASS_SCALE: usize = 2;
/// Where the long window's bins give way to the short one's, in Hz: wholly
/// below the first, not at all above the second.
const BASS_SPLIT: (f32, f32) = (200.0, 400.0);

/// One spectrum's bins, read for bands.
struct Bins {
    freqs: Vec<f32>,
    vals: Vec<f32>,
    bin_hz: f32,
    min_width: f32,
}

impl Bins {
    fn new(data: &[(f32, f32)], bin_hz: f32, min_width: f32) -> Self {
        Self {
            freqs: data.iter().map(|&(freq, _)| freq).collect(),
            vals: data.iter().map(|&(_, value)| value).collect(),
            bin_hz,
            min_width,
        }
    }

    /// The band at `freq`: the loudest bin within 2.5% of it, its window at
    /// least a bin wide, or a band can fall between two bins and read nothing
    /// ever. Where a band's window holds a bin or so, bands lie closer
    /// together than the bins, and runs of them read the same one: flat tops
    /// ending in a step. There it reads a smooth curve through the bins at its
    /// own frequency instead, blending into the loudest-bin reading by where
    /// windows hold three.
    fn read(&self, freq: f32) -> f32 {
        if self.freqs.is_empty() {
            return 0.0;
        }
        let width = (freq * 0.05).max(self.min_width).max(self.bin_hz * 1.01);
        let (lo, hi) = (freq - width / 2.0, freq + width / 2.0);
        let start = self.freqs.partition_point(|&f| f < lo);
        let loudest = self.freqs[start..]
            .iter()
            .zip(&self.vals[start..])
            .take_while(|&(&f, _)| f <= hi)
            .map(|(_, &v)| v)
            .fold(0.0, f32::max);
        let curve = ((3.0 - width / self.bin_hz) / 2.0).clamp(0.0, 1.0);
        if curve > 0.0 {
            let between = curve_through_bins(&self.vals, (freq - self.freqs[0]) / self.bin_hz);
            between * curve + loudest * (1.0 - curve)
        } else {
            loudest
        }
    }
}

/// Heights go as amplitude to this power. The square root, as they did, drew
/// a tone ten times louder than its surroundings three times as tall: one
/// overtone stood as a needle far above the rest. At this power it stands
/// about twice as tall; over real songs the tallest column stands 1.9 times
/// the average rather than 2.5, needles are three in four fewer, and loud
/// moments still stand out. `HEIGHT_GAIN` keeps the average height as it was.
const HEIGHT_CURVE: f32 = 0.35;
const HEIGHT_GAIN: f32 = 0.48;

/// The value at `at`, counted in bins, on a Catmull–Rom curve through the
/// bins' `values`: it passes through every bin, and rounds over a peak
/// that falls between two rather than meeting it in a corner.
fn curve_through_bins(values: &[f32], at: f32) -> f32 {
    let last = values.len().saturating_sub(1) as isize;
    let k = at.floor() as isize;
    let t = at - k as f32;
    let v = |i: isize| values[i.clamp(0, last) as usize];
    let (p0, p1, p2, p3) = (v(k - 1), v(k), v(k + 1), v(k + 2));
    let value = 0.5
        * (2.0 * p1
            + (p2 - p0) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
            + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t * t * t);
    value.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::traits::{Producer, Split};
    use ringbuf::{HeapProd, HeapRb};
    use std::time::Duration;

    const RATE: u32 = 48_000;

    fn analyzer() -> (HeapProd<f32>, SpectrumAnalyzer<ringbuf::HeapCons<f32>>) {
        let (prod, cons) = HeapRb::<f32>::new(RATE as usize).split();
        (prod, SpectrumAnalyzer::new(cons, 4096, RATE))
    }

    /// Plays `seconds` of audio that arrives `lump` samples at a time, at
    /// `speed` times real time, ticking every 10 ms like the worker does.
    /// Returns where the window ended after each tick, `None` when the tick
    /// produced no spectrum.
    fn play(
        prod: &mut HeapProd<f32>,
        analyzer: &mut SpectrumAnalyzer<ringbuf::HeapCons<f32>>,
        start: Instant,
        lump: usize,
        speed: f64,
        seconds: f64,
    ) -> Vec<Option<u64>> {
        let lump_every = lump as f64 / (RATE as f64 * speed);
        let mut next_lump = 0.0;
        let mut ends = Vec::new();
        let mut t = 0.0;
        while t < seconds {
            while next_lump <= t {
                let samples: Vec<f32> = (0..lump).map(|i| (i as f32 * 0.1).sin()).collect();
                prod.push_slice(&samples);
                next_lump += lump_every;
            }
            let now = start + Duration::from_secs_f64(t);
            ends.push(
                analyzer
                    .next_spectrum_at(now)
                    .map(|_| analyzer.window_end()),
            );
            t += 0.010;
        }
        ends
    }

    #[test]
    fn audio_arriving_in_big_lumps_still_gives_a_spectrum_every_tick() {
        let (mut prod, mut analyzer) = analyzer();
        // 8192 frames at a time, as the device pulled with cpal 0.18.
        let ends = play(&mut prod, &mut analyzer, Instant::now(), 8192, 1.0, 6.0);
        let settled = &ends[300..];
        assert!(
            settled.iter().all(Option::is_some),
            "the picture froze between lumps: {settled:?}"
        );
        for pair in settled.windows(2) {
            let step = pair[1].unwrap() as f64 - pair[0].unwrap() as f64;
            assert!(
                (400.0..=560.0).contains(&step),
                "the window jumped {step} samples in one 10 ms tick"
            );
        }
    }

    #[test]
    fn the_window_stays_behind_the_newest_audio_by_about_a_lump_and_a_half() {
        let (mut prod, mut analyzer) = analyzer();
        play(&mut prod, &mut analyzer, Instant::now(), 4096, 1.0, 6.0);
        let backlog = analyzer.received as f64 - analyzer.playhead;
        assert!(
            (4096.0..=2.0 * 4096.0 + 480.0).contains(&backlog),
            "backlog {backlog}"
        );
    }

    #[test]
    fn faster_playback_is_followed_without_stalling() {
        let (mut prod, mut analyzer) = analyzer();
        let ends = play(&mut prod, &mut analyzer, Instant::now(), 2048, 1.5, 15.0);
        let settled = &ends[1000..];
        assert!(settled.iter().all(Option::is_some));
        let speed = analyzer.speed / RATE as f64;
        assert!((1.4..1.6).contains(&speed), "speed {speed}");
    }

    #[test]
    fn a_pause_starts_the_pacing_over_instead_of_racing() {
        let (mut prod, mut analyzer) = analyzer();
        let start = Instant::now();
        play(&mut prod, &mut analyzer, start, 4096, 1.0, 3.0);
        // Paused for ten seconds: no ticks, and nothing to play.
        let resumed = start + Duration::from_secs(13);
        let ends = play(&mut prod, &mut analyzer, resumed, 4096, 1.0, 3.0);
        assert!(ends[100..].iter().all(Option::is_some));
        let backlog = analyzer.received as f64 - analyzer.playhead;
        assert!(backlog >= 4096.0, "backlog {backlog}");
    }

    #[test]
    fn after_a_restart_only_audio_from_then_on_is_drawn() {
        let (mut prod, mut analyzer) = analyzer();
        let start = Instant::now();
        play(&mut prod, &mut analyzer, start, 2048, 1.0, 1.0);
        // Audio from before a seek, waiting to be read.
        prod.push_slice(&[0.9; 4096]);
        analyzer.restart();
        assert!(analyzer.history.is_empty());
        assert!(
            analyzer
                .next_spectrum_at(start + Duration::from_millis(1010))
                .is_none(),
            "nothing to draw until a window of new audio has come"
        );
        let ends = play(
            &mut prod,
            &mut analyzer,
            start + Duration::from_secs(2),
            2048,
            1.0,
            1.0,
        );
        assert!(ends[50..].iter().all(Option::is_some));
    }

    #[test]
    fn no_band_is_left_empty_at_48_khz() {
        let (mut prod, cons) = HeapRb::<f32>::new(RATE as usize).split();
        let mut analyzer = SpectrumAnalyzer::new(cons, 4096, RATE);
        // White noise: something at every frequency.
        let mut seed = 0x2545_F491_u32;
        let noise: Vec<f32> = (0..16384)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as f32 / u32::MAX as f32 - 0.5
            })
            .collect();
        prod.push_slice(&noise);
        let bands = analyzer.next_spectrum_at(Instant::now()).unwrap();
        let empty: Vec<usize> = (0..bands.len()).filter(|&i| bands[i] == 0.0).collect();
        assert!(empty.is_empty(), "bands that read nothing: {empty:?}");
    }

    #[test]
    fn the_curve_through_the_bins_meets_them_and_runs_smoothly_between() {
        let bins = [0.0, 1.0, 4.0, 9.0, 4.0, 0.0];
        for (k, &v) in bins.iter().enumerate() {
            assert!((curve_through_bins(&bins, k as f32) - v).abs() < 1e-6);
        }
        // A straight run stays straight between the bins.
        let line = [0.0, 1.0, 2.0, 3.0];
        assert!((curve_through_bins(&line, 1.5) - 1.5).abs() < 1e-6);
        // Never below silence, however the curve swings.
        assert!((0..50).all(|i| curve_through_bins(&bins, i as f32 / 10.0) >= 0.0));
    }

    #[test]
    fn test_spectrum_analyzer_incomplete_frame() {
        let (mut prod, mut analyzer) = analyzer();

        // Push fewer than FFT_SIZE samples
        for _ in 0..1024 {
            prod.try_push(0.0).unwrap();
        }

        // Should return None because it doesn't have enough samples
        assert!(analyzer.next_spectrum().is_none());
    }

    #[test]
    fn test_spectrum_analyzer_full_frame() {
        let (mut prod, mut analyzer) = analyzer();
        prod.push_slice(&[0.5; 8192]);
        let data = analyzer
            .next_spectrum_at(Instant::now())
            .expect("a window's worth is waiting");
        assert_eq!(data.len(), NUM_BINS);
    }

    #[test]
    fn test_spectrum_analyzer_nan_handling() {
        let (mut prod, mut analyzer) = analyzer();
        let start = Instant::now();

        // Push NaNs and Infinities
        for i in 0..8192 {
            let val = if i % 2 == 0 { f32::NAN } else { f32::INFINITY };
            prod.try_push(val).unwrap();
        }

        // The FFT library might return an error or skip it. We just verify it doesn't panic.
        analyzer.next_spectrum_at(start);
        let result = analyzer.next_spectrum_at(start + Duration::from_millis(10));
        if let Some(data) = result {
            assert_eq!(data.len(), NUM_BINS);
        }
    }
}
