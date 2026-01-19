use std::{process::Command, str::FromStr};

use crate::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VisualizerMode {
    Spectrum,
    #[default]
    AlbumArt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpectrumStyle {
    Cava,
    Default,
    Bar,
    Particle,
}

pub fn is_cava_available() -> bool {
    Command::new("cava").arg("--version").output().is_ok()
}

impl Default for SpectrumStyle {
    fn default() -> Self {
        if is_cava_available() {
            tracing::info!("Cava installation detected. Using it for spectrum");
            SpectrumStyle::Cava
        } else {
            tracing::info!("Cava installation not detected. Switching to manual mode.");
            tracing::info!("Install Cava for better spectrum visualization");
            SpectrumStyle::Default
        }
    }
}

impl SpectrumStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpectrumStyle::Cava => "Cava",
            SpectrumStyle::Default => "Default",
            SpectrumStyle::Bar => "Bar",
            SpectrumStyle::Particle => "Particle",
        }
    }
}

impl FromStr for SpectrumStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cava" => Ok(SpectrumStyle::Cava),
            "default" | "def" => Ok(SpectrumStyle::Default),
            "bar" => Ok(SpectrumStyle::Bar),
            "particle" | "part" | "p" => Ok(SpectrumStyle::Particle),
            _ => Err(()),
        }
    }
}

pub fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if !state.should_draw_spectrum() {
        return;
    }

    if state.spectrum_raw.len() < 512 {
        state.spectrum_raw.resize(512, 0.0);
    }

    let count = state.spectrum_source.get_latest(&mut state.spectrum_raw);

    if count > 0 {
        state.spectrum_raw.truncate(count);
    }

    let len = state.spectrum_raw.len();
    if state.spectrum_smooth.len() != len {
        state.spectrum_smooth.resize(len, 0.0);
    }
    if state.spectrum_peaks.len() != len {
        state.spectrum_peaks.resize(len, 0.0);
    }

    if *state.spectrum_style() == SpectrumStyle::Cava {
        state.spectrum_smooth.copy_from_slice(&state.spectrum_raw);
        for i in 0..len {
            if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
                state.spectrum_peaks[i] = state.spectrum_smooth[i];
            } else {
                state.spectrum_peaks[i] -= 2.0 * dt;
                if state.spectrum_peaks[i] < 0.0 {
                    state.spectrum_peaks[i] = 0.0;
                }
            }
        }
        return;
    }

    let gravity = 4.0;
    let smoothing_speed = 15.0;

    for i in 0..len {
        let target = state.spectrum_raw[i];
        let diff = target - state.spectrum_smooth[i];
        state.spectrum_smooth[i] += diff * (smoothing_speed * dt).min(1.0);

        if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
            state.spectrum_peaks[i] = state.spectrum_smooth[i];
        } else {
            state.spectrum_peaks[i] -= gravity * dt;
            if state.spectrum_peaks[i] < 0.0 {
                state.spectrum_peaks[i] = 0.0;
            }
        }
    }
}
