/// Center frequencies for the 10-band parametric EQ (Hz).
pub const EQ_BAND_FREQS: [f32; 10] = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

/// Labels for the 10-band parametric EQ.
pub const EQ_BAND_LABELS: [&str; 10] = ["32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];

/// Total number of DSP controls (10 EQ bands + 9 sliders).
pub const DSP_CONTROL_COUNT: usize = 19;

#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32,  // range 0.0 to 1.5
    pub speed: f32,   // range 0.5 to 2.0
    pub bass: f32,    // range 0.1 to 5.0
    pub treble: f32,  // range 0.0 to 5.0
    pub reverb: f32,  // range 0.0 to 1.0
    pub echo: f32,    // range 0.0 to 0.9
    pub pan: f32,     // range -1.0 (full left) to 1.0 (full right)
    pub lowpass: f32,  // range 200.0 to 20000.0 Hz (20000 = bypass)
    pub highpass: f32, // range 20.0 to 5000.0 Hz (20 = bypass)
    pub eq_bands: [f32; 10], // range -12.0 to +12.0 dB per band
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            bass: 1.0,
            treble: 1.0,
            reverb: 0.0,
            echo: 0.0,
            pan: 0.0,
            lowpass: 20000.0,
            highpass: 20.0,
            eq_bands: [0.0; 10],
        }
    }
}
