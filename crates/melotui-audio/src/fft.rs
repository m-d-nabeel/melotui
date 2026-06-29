use ringbuf::traits::Consumer;
use spectrum_analyzer::{
    FrequencyLimit, samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window,
};
use std::collections::BTreeMap;

const NUM_BINS: usize = 512;
const MIN_FREQ: f32 = 30.0; // catch sub-bass
const MAX_FREQ: f32 = 18_000.0;

pub struct SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    consumer: C,
    buffer: Vec<f32>,
    fft_size: usize,
    sample_rate: u32,
}

impl<C> SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    pub fn new(consumer: C, fft_size: usize, sample_rate: u32) -> Self {
        Self {
            consumer,
            buffer: Vec::with_capacity(fft_size),
            fft_size,
            sample_rate,
        }
    }

    pub fn into_consumer(self) -> C {
        self.consumer
    }

    pub fn next_spectrum(&mut self) -> Option<Vec<f32>> {
        while let Some(sample) = self.consumer.try_pop() {
            if self.buffer.len() < self.fft_size {
                self.buffer.push(sample);
            }
        }

        // wait for full buffer
        if self.buffer.len() < self.fft_size {
            return None;
        }

        let hann_window = hann_window(&self.buffer);
        let spectrum_result = samples_fft_to_spectrum(
            &hann_window,
            self.sample_rate,
            FrequencyLimit::Range(20.0, 22_050.0),
            Some(&divide_by_N),
        );

        // shift buffer for next frame (75% overlap)
        let drain_amt = self.fft_size - 1024;
        if drain_amt < self.buffer.len() {
            self.buffer.drain(0..drain_amt);
        } else {
            self.buffer.clear();
        }

        if let Ok(spectrum) = spectrum_result {
            let data = spectrum.to_map();
            // map linear spectrum to log bins for UI
            let output = self.fill_log_bins(&data, NUM_BINS);
            return Some(output);
        }

        None
    }

    fn fill_log_bins(&self, data: &BTreeMap<u32, f32>, bin_count: usize) -> Vec<f32> {
        let mut output = vec![0.0; bin_count];

        let log_min = MIN_FREQ.ln();
        let log_max = MAX_FREQ.ln();
        let log_range = log_max - log_min;

        // sparse map to dense vectors
        let freqs: Vec<f32> = data.keys().map(|&k| k as f32).collect();
        let vals: Vec<f32> = data.values().copied().collect();

        if freqs.is_empty() {
            return output;
        }

        (0..bin_count).for_each(|i| {
            // target frequency
            let norm_pos = i as f32 / (bin_count as f32 - 1.0);
            let target_freq = (log_min + norm_pos * log_range).exp();

            // frequency window
            let window_width = (target_freq * 0.05).max(11.0);
            let start_f = target_freq - window_width / 2.0;
            let end_f = target_freq + window_width / 2.0;

            // max value in window
            let mut max_val = 0.0;

            // find start index
            let start_idx = match freqs.binary_search_by(|f| f.partial_cmp(&start_f).unwrap()) {
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
    use ringbuf::HeapRb;
    use ringbuf::traits::{Producer, Split};

    #[test]
    fn test_spectrum_analyzer_incomplete_frame() {
        let rb = HeapRb::<f32>::new(8192);
        let (mut prod, cons) = rb.split();
        let mut analyzer = SpectrumAnalyzer::new(cons, 4096, 44100);

        // Push fewer than FFT_SIZE samples
        for _ in 0..1024 {
            prod.try_push(0.0).unwrap();
        }

        // Should return None because it doesn't have enough samples
        assert!(analyzer.next_spectrum().is_none());
    }

    #[test]
    fn test_spectrum_analyzer_full_frame() {
        let rb = HeapRb::<f32>::new(8192);
        let (mut prod, cons) = rb.split();
        let mut analyzer = SpectrumAnalyzer::new(cons, 4096, 44100);

        // Push enough for a full frame
        for _ in 0..4096 {
            prod.try_push(0.5).unwrap();
        }

        let spectrum = analyzer.next_spectrum();
        assert!(spectrum.is_some());
        let data = spectrum.unwrap();
        assert_eq!(data.len(), NUM_BINS);
    }

    #[test]
    fn test_spectrum_analyzer_nan_handling() {
        let rb = HeapRb::<f32>::new(8192);
        let (mut prod, cons) = rb.split();
        let mut analyzer = SpectrumAnalyzer::new(cons, 4096, 44100);

        // Push NaNs and Infinities
        for i in 0..4096 {
            let val = if i % 2 == 0 { f32::NAN } else { f32::INFINITY };
            prod.try_push(val).unwrap();
        }

        // The FFT library might return an error or skip it. We just verify it doesn't panic.
        let result = analyzer.next_spectrum();
        // It's acceptable for it to be None or Some with invalid floats, as long as it handles the failure gracefully.
        // It actually returns None if FFT fails.
        if let Some(data) = result {
            assert_eq!(data.len(), NUM_BINS);
        }
    }
}
