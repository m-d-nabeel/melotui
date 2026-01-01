use ringbuf::traits::Producer;
use rodio::Source;
use std::time::Duration;

/// Wraps a Rodio source and copies samples to a ring buffer.
/// We make this generic over P so it accepts whatever specific Producer type
/// ringbuf creates (HeapProducer, StackProducer, etc.).
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
            // In ringbuf 0.4+, use try_push for the trait
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
    // FIX: Rodio 0.21+ uses current_span_len, not current_frame_len
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
}
