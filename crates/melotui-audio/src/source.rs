use fundsp::buffer::BufferVec;
use fundsp::hacker32::*;
use rodio::Source;
use rodio::source::SeekError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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

        unit.reset();
        unit.set_sample_rate(source.sample_rate() as f64);

        let input_buffer = BufferVec::new(input_channels);
        let output_buffer = BufferVec::new(Self::OUTPUT_CHANNELS); // Always output stereo

        Self {
            source,
            unit,
            input_channels,
            input_buffer,
            output_buffer,
            output_queue: Vec::with_capacity(block_size * Self::OUTPUT_CHANNELS),
            current_idx: 0,
            bypass,
        }
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

        if self.bypass.load(Ordering::Relaxed) {
            let input_ref = self.input_buffer.buffer_ref();
            for i in 0..frames_read {
                if self.input_channels == 1 {
                    let sample = input_ref.channel_f32(0)[i];
                    for _ in 0..Self::OUTPUT_CHANNELS {
                        self.output_queue.push(sample);
                    }
                } else {
                    for ch in 0..Self::OUTPUT_CHANNELS {
                        let ch_idx = if ch < self.input_channels { ch } else { 0 };
                        let sample = input_ref.channel_f32(ch_idx)[i];
                        self.output_queue.push(sample);
                    }
                }
            }
            return true;
        }

        let input_ref = self.input_buffer.buffer_ref();
        let mut output_mut = self.output_buffer.buffer_mut();

        self.unit.process(frames_read, &input_ref, &mut output_mut);

        let output_ref = self.output_buffer.buffer_ref();

        for i in 0..frames_read {
            for ch in 0..Self::OUTPUT_CHANNELS {
                let sample = output_ref.channel_f32(ch)[i];
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
        self.source.current_span_len()
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

        // reset dsp
        self.unit.reset();
        self.unit.set_sample_rate(self.source.sample_rate() as f64);

        Ok(())
    }
}
