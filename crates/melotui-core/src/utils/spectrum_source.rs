/// Interface for spectrum data providers.
pub trait SpectrumSource: Send + Sync {
    /// Copies latest spectrum data into `out`.
    fn get_latest(&self, out: &mut [f32]) -> usize;
}

/// No-op implementation for disabled visualizer.
pub struct NoOpSpectrum;

impl SpectrumSource for NoOpSpectrum {
    fn get_latest(&self, _out: &mut [f32]) -> usize {
        0
    }
}
