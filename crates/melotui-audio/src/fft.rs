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
            window: vec![0.0; fft_size],
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

        // Keep only what the window can still reach.
        let end = self.window_end();
        let keep_from = end.saturating_sub(self.fft_size as u64);
        if keep_from > self.first {
            let drop = ((keep_from - self.first) as usize).min(self.history.len());
            self.history.drain(..drop);
            self.first += drop as u64;
        }

        let start = end.checked_sub(self.fft_size as u64)?;
        if end == old_end || start < self.first {
            return None;
        }
        let offset = (start - self.first) as usize;
        for (slot, sample) in self
            .window
            .iter_mut()
            .zip(self.history.range(offset..offset + self.fft_size))
        {
            *slot = *sample;
        }

        let hann_window = hann_window(&self.window);
        let spectrum_result = samples_fft_to_spectrum(
            &hann_window,
            self.sample_rate,
            FrequencyLimit::Range(20.0, 22_050.0),
            Some(&divide_by_N),
        );

        if let Ok(spectrum) = spectrum_result {
            // Exact frequencies: `to_map` keys them by whole hertz, which
            // moved bins out of the narrow bass bands they belonged in.
            let data: Vec<(f32, f32)> = spectrum
                .data()
                .iter()
                .map(|(freq, value)| (freq.val(), value.val()))
                .collect();
            // map linear spectrum to log bins for UI
            let output = self.fill_log_bins(&data, NUM_BINS);
            return Some(output);
        }

        None
    }

    /// The sample the analysis window ends at.
    fn window_end(&self) -> u64 {
        self.playhead.max(0.0) as u64
    }

    fn fill_log_bins(&self, data: &[(f32, f32)], bin_count: usize) -> Vec<f32> {
        let bin_hz = self.sample_rate as f32 / self.fft_size as f32;
        let mut output = vec![0.0; bin_count];

        let log_min = MIN_FREQ.ln();
        let log_max = MAX_FREQ.ln();
        let log_range = log_max - log_min;

        // sparse map to dense vectors
        let freqs: Vec<f32> = data.iter().map(|&(freq, _)| freq).collect();
        let vals: Vec<f32> = data.iter().map(|&(_, value)| value).collect();

        if freqs.is_empty() {
            return output;
        }

        (0..bin_count).for_each(|i| {
            // target frequency
            let norm_pos = i as f32 / (bin_count as f32 - 1.0);
            let target_freq = (log_min + norm_pos * log_range).exp();

            // frequency window
            // At least a bin wide, or a bass band can fall between two bins
            // and read nothing ever: at 48 kHz, bins are 11.7 Hz apart, and
            // six bands from 52 to 228 Hz were always empty.
            let window_width = (target_freq * 0.05).max(11.0).max(bin_hz * 1.01);
            let start_f = target_freq - window_width / 2.0;
            let end_f = target_freq + window_width / 2.0;

            // max value in window
            let mut max_val = 0.0;

            // find start index
            let start_idx = match freqs.binary_search_by(|f| f.total_cmp(&start_f)) {
                Ok(idx) => idx,
                Err(idx) => idx,
            };

            for j in start_idx..freqs.len() {
                if freqs[j] > end_f {
                    break;
                }
                if vals[j] > max_val {
                    max_val = vals[j];
                }
            }

            // scale value based on position (highs usually quieter)
            output[i] = max_val.sqrt() * (1.0 + norm_pos);
        });

        output
    }
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
