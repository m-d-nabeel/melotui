/// Interface for any component that provides spectrum data.
/// This decouples the UI from the Audio Engine.
pub trait SpectrumSource: Send + Sync {
    /// Copies the latest available spectrum data into `out`.
    /// Returns the number of bins copied.
    fn get_latest(&self, out: &mut [f32]) -> usize;
}

/// No-op implementation for when the visualizer is disabled/initializing.
pub struct NoOpSpectrum;

impl SpectrumSource for NoOpSpectrum {
    fn get_latest(&self, _out: &mut [f32]) -> usize {
        0
    }
}
