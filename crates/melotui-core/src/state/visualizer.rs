use std::{process::Command, str::FromStr};

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
