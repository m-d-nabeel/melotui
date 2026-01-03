use fundsp::buffer::BufferVec;
use fundsp::hacker32::*;
use rodio::Source;
use std::time::Duration;

/// Wrapper to make FunDSP graphs compatible with Rodio.
/// Processes audio in blocks of 64 samples (MAX_BUFFER_SIZE) for efficiency.
pub struct FundspSource<S> {
    source: S,
    unit: Box<dyn AudioUnit>,
    channels: usize,

    // Block Processing State
    input_buffer: BufferVec,
    output_buffer: BufferVec,

    // Queue for interleaved samples to yield to Rodio
    output_queue: Vec<f32>,
    current_idx: usize,
}

impl<S> FundspSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(source: S, unit: Box<dyn AudioUnit>) -> Self {
        let channels = source.channels() as usize;
        let mut unit = unit;
        let block_size = 64;

        unit.reset();
        unit.set_sample_rate(source.sample_rate() as f64);

        let input_buffer = BufferVec::new(channels);
        let output_buffer = BufferVec::new(channels);

        Self {
            source,
            unit,
            channels,
            input_buffer,
            output_buffer,
            output_queue: Vec::with_capacity(block_size * channels),
            current_idx: 0,
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
            for ch in 0..self.channels {
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

        let input_ref = self.input_buffer.buffer_ref();
        let mut output_mut = self.output_buffer.buffer_mut();

        self.unit.process(frames_read, &input_ref, &mut output_mut);

        let output_ref = self.output_buffer.buffer_ref();

        for i in 0..frames_read {
            for ch in 0..self.channels {
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
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}
