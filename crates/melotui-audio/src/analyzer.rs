use ringbuf::traits::Consumer;
use spectrum_analyzer::{
    FrequencyLimit, samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window,
};
use std::collections::BTreeMap;

const FFT_SIZE: usize = 4096;
const NUM_BINS: usize = 512;
const MIN_FREQ: f32 = 30.0; // to catch sub-bass
const MAX_FREQ: f32 = 18_000.0;

pub struct SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    consumer: C,
    buffer: Vec<f32>,
}

impl<C> SpectrumAnalyzer<C>
where
    C: Consumer<Item = f32>,
{
    pub fn new(consumer: C) -> Self {
        Self {
            consumer,
            buffer: Vec::with_capacity(FFT_SIZE),
        }
    }

    pub fn next_spectrum(&mut self) -> Option<Vec<f32>> {
        while let Some(sample) = self.consumer.try_pop() {
            if self.buffer.len() < FFT_SIZE {
                self.buffer.push(sample);
            }
        }

        if self.buffer.len() < FFT_SIZE {
            return None;
        }

        let hann_window = hann_window(&self.buffer);
        let spectrum_result = samples_fft_to_spectrum(
            &hann_window,
            44100,
            FrequencyLimit::Range(20.0, 22_050.0),
            Some(&divide_by_N),
        );

        let drain_amt = FFT_SIZE - 1024;
        if drain_amt < self.buffer.len() {
            self.buffer.drain(0..drain_amt);
        } else {
            self.buffer.clear();
        }

        if let Ok(spectrum) = spectrum_result {
            let data = spectrum.to_map();
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

        // Convert sparse map to dense vectors for easier searching
        let freqs: Vec<f32> = data.keys().map(|&k| k as f32).collect();
        let vals: Vec<f32> = data.values().copied().collect();

        if freqs.is_empty() {
            return output;
        }

        (0..bin_count).for_each(|i| {
            // Target frequency for this bin
            let norm_pos = i as f32 / (bin_count as f32 - 1.0);
            let target_freq = (log_min + norm_pos * log_range).exp();

            // Define a window around the target freq.
            // At low freqs, the window is small. At high freqs, it's large.
            // We ensure the window is at least the size of one FFT bin step (~10Hz)
            let window_width = (target_freq * 0.05).max(11.0);
            let start_f = target_freq - window_width / 2.0;
            let end_f = target_freq + window_width / 2.0;

            // Find max value within this window
            let mut max_val = 0.0;

            // Efficiently find start index
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

            output[i] = max_val.sqrt() * (1.0 + norm_pos);
        });

        output
    }
}
