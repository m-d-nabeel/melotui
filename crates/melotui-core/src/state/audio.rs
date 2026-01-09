#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32, // 0.0 - 1.5
    pub speed: f32,  // 0.5 - 2.0
    pub bass: f32,   // 0.1 - 5.0
    pub treble: f32, // 0.0 - 5.0
    pub reverb: f32, // 0.0 - 1.0
    pub echo: f32,   // 0.0 - 0.9
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
        }
    }
}
