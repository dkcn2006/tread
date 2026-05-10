use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub default_mode: Option<String>,
    #[serde(default)]
    pub display_lines: Option<usize>,
    #[serde(default)]
    pub templates: Vec<TemplatePreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePreset {
    pub name: String,
    pub template: String,
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn find_template(&self, name: &str) -> Option<&str> {
        self.templates
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.template.as_str())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("terminal-read")
            .join("config.toml")
    }
}
