/// Center frequencies for the 10-band parametric EQ (Hz).
pub const EQ_BAND_FREQS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Labels for the 10-band parametric EQ.
pub const EQ_BAND_LABELS: [&str; 10] = [
    "32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];

/// Total number of DSP controls.
/// EQ bands 0-9, then 3-column grid:
///   Sound:   Volume(10), Speed(11), Bass(12), Treble(13), Exciter(21), Bitcrush(22)
///   Spatial: Pan(14), AP Rate(15), AP Depth(16), Width(23), Karaoke(24), PingPong(26)
///   Effects: Reverb(17), Echo(18), Lo-pass(19), Hi-pass(20), Wobble(25), Phaser(27)
pub const DSP_CONTROL_COUNT: usize = 28;

/// Grid column layouts for spatial navigation.
pub const GRID_COL_SOUND: [usize; 6] = [10, 11, 12, 13, 21, 22];
pub const GRID_COL_SPATIAL: [usize; 6] = [14, 15, 16, 23, 24, 26];
pub const GRID_COL_EFFECTS: [usize; 6] = [17, 18, 19, 20, 25, 27];

#[derive(Debug, Clone)]
pub struct AudioState {
    pub volume: f32,         // range 0.0 to 1.5
    pub speed: f32,          // range 0.5 to 2.0
    pub bass: f32,           // range 0.1 to 5.0
    pub treble: f32,         // range 0.1 to 5.0
    pub reverb: f32,         // range 0.0 to 1.0
    pub echo: f32,           // range 0.0 to 0.9
    pub pan: f32,            // range -1.0 (full left) to 1.0 (full right)
    pub lowpass: f32,        // range 200.0 to 20000.0 Hz (20000 = bypass)
    pub highpass: f32,       // range 20.0 to 5000.0 Hz (20 = bypass)
    pub auto_pan_rate: f32,  // LFO speed in Hz, 0.0 = off, range 0.0 to 2.0
    pub auto_pan_depth: f32, // LFO depth, range 0.0 to 1.0
    pub eq_bands: [f32; 10], // range -12.0 to +12.0 dB per band
    pub exciter: f32,        // range 0.0 to 5.0
    pub bitcrush: f32,       // range 0.0 to 1.0
    pub width: f32,          // range 0.0 to 3.0 (1.0 = normal)
    pub karaoke: f32,        // range 0.0 to 1.0
    pub wobble: f32,         // range 0.0 to 1.0
    pub ping_pong: f32,      // range 0.0 to 1.0
    pub phaser: f32,         // range 0.0 to 1.0
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
            auto_pan_rate: 0.0,
            auto_pan_depth: 0.0,
            eq_bands: [0.0; 10],
            exciter: 0.0,
            bitcrush: 0.0,
            width: 1.0,
            karaoke: 0.0,
            wobble: 0.0,
            ping_pong: 0.0,
            phaser: 0.0,
        }
    }
}

impl AudioState {
    pub fn sanitize(&mut self) {
        let defaults = Self::default();
        self.volume = clamp_finite(self.volume, defaults.volume, 0.0, 1.5);
        self.speed = clamp_finite(self.speed, defaults.speed, 0.5, 2.0);
        self.bass = clamp_finite(self.bass, defaults.bass, 0.1, 5.0);
        self.treble = clamp_finite(self.treble, defaults.treble, 0.1, 5.0);
        self.reverb = clamp_finite(self.reverb, defaults.reverb, 0.0, 1.0);
        self.echo = clamp_finite(self.echo, defaults.echo, 0.0, 0.9);
        self.pan = clamp_finite(self.pan, defaults.pan, -1.0, 1.0);
        self.lowpass = clamp_finite(self.lowpass, defaults.lowpass, 200.0, 20000.0);
        self.highpass = clamp_finite(self.highpass, defaults.highpass, 20.0, 5000.0);
        self.auto_pan_rate = clamp_finite(self.auto_pan_rate, defaults.auto_pan_rate, 0.0, 2.0);
        self.auto_pan_depth = clamp_finite(self.auto_pan_depth, defaults.auto_pan_depth, 0.0, 1.0);
        for band in &mut self.eq_bands {
            *band = clamp_finite(*band, 0.0, -12.0, 12.0);
        }
        self.exciter = clamp_finite(self.exciter, defaults.exciter, 0.0, 5.0);
        self.bitcrush = clamp_finite(self.bitcrush, defaults.bitcrush, 0.0, 1.0);
        self.width = clamp_finite(self.width, defaults.width, 0.0, 3.0);
        self.karaoke = clamp_finite(self.karaoke, defaults.karaoke, 0.0, 1.0);
        self.wobble = clamp_finite(self.wobble, defaults.wobble, 0.0, 1.0);
        self.ping_pong = clamp_finite(self.ping_pong, defaults.ping_pong, 0.0, 1.0);
        self.phaser = clamp_finite(self.phaser, defaults.phaser, 0.0, 1.0);
    }
}

fn clamp_finite(value: f32, default: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}
