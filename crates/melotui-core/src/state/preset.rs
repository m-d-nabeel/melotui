use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::state::PersistedDspParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DspPreset {
    pub name: String,
    pub params: PersistedDspParams,
}

impl DspPreset {
    pub fn presets_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("melotui")
            .join("presets");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir)
    }

    pub fn list() -> Result<Vec<String>> {
        let dir = Self::presets_dir()?;
        let mut names = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn load(name: &str) -> Result<Self> {
        let path = Self::presets_dir()?.join(format!("{}.toml", name));
        if !path.exists() {
            anyhow::bail!("Preset '{}' does not exist", name);
        }
        let content = std::fs::read_to_string(&path)?;
        let preset: Self = toml::from_str(&content)?;
        Ok(preset)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::presets_dir()?.join(format!("{}.toml", self.name));
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn delete(name: &str) -> Result<()> {
        let path = Self::presets_dir()?.join(format!("{}.toml", name));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsp_preset_serialization() {
        let preset = DspPreset {
            name: "Bass Boost".to_string(),
            params: PersistedDspParams {
                bass: 5.0,
                treble: -2.0,
                eq_bands: [2.0, 3.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            },
        };

        let toml_str = toml::to_string_pretty(&preset).expect("Serialize failed");
        let deserialized: DspPreset = toml::from_str(&toml_str).expect("Deserialize failed");

        assert_eq!(deserialized.name, "Bass Boost");
        assert_eq!(deserialized.params.bass, 5.0);
        assert_eq!(deserialized.params.treble, -2.0);
        assert_eq!(deserialized.params.eq_bands[2], 4.0);
    }
}
