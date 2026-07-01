use ringbuf::traits::Producer;
use rodio::{Source, source::SeekError};
use std::time::Duration;

/// copy samples -> ring buffer
pub struct SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    pub input: I,
    pub producer: P,
    pub batch: [f32; 64],
    pub batch_pos: usize,
}

impl<I, P> Iterator for SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next()?;

        self.batch[self.batch_pos] = sample;
        self.batch_pos += 1;

        if self.batch_pos >= 64 {
            let _ = self.producer.push_slice(&self.batch);
            self.batch_pos = 0;
        }

        Some(sample)
    }
}

impl<I, P> Source for SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }
    fn channels(&self) -> u16 {
        self.input.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)
    }
}
