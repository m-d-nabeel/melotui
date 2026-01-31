use ringbuf::traits::Producer;
use rodio::{Source, source::SeekError};
use std::time::Duration;

/// Rodio source wrapper that copies samples into a ring buffer.
pub struct SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    pub input: I,
    pub producer: P,
}

impl<I, P> Iterator for SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next();
        if let Some(s) = sample {
            let _ = self.producer.try_push(s);
        }
        sample
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
