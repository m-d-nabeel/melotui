use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VisualizerMode {
    Spectrum,
    #[default]
    AlbumArt,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SpectrumStyle {
    Cava,
    #[default]
    Default,
    Bar,
    Particle,
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
