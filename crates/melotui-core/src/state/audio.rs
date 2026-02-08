#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32, // range 0.0 to 1.5
    pub speed: f32,  // range 0.5 to 2.0
    pub bass: f32,   // range 0.1 to 5.0
    pub treble: f32, // range 0.0 to 5.0
    pub reverb: f32, // range 0.0 to 1.0
    pub echo: f32,   // range 0.0 to 0.9
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
