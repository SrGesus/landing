use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "Config::default_path")]
    path: PathBuf,
    #[serde(default = "Config::default_endpoint")]
    endpoint: String,
    #[serde(default = "Config::default_index_word")]
    index_word: String,
    include: Option<String>,

    #[serde(default)]
    templates: FileConfig,
    #[serde(default)]
    scripts: FileConfig,
    #[serde(default)]
    files: FileConfig,
    #[serde(default)]
    tailwind: TailwindConfig,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let config_str = std::fs::read_to_string(path.as_ref())?;
        Ok(toml::from_str(&config_str)?)
    }

    pub fn get_index_word(&self) -> &str {
        &self.index_word
    }

    pub fn get_include(&self) -> &Option<String> {
        &self.include
    }

    pub fn get_templates_path(&self) -> &PathBuf {
        self.templates.path.as_ref().unwrap_or(&self.path)
    }

    pub fn get_templates_endpoint(&self) -> &str {
        self.templates.endpoint.as_ref().unwrap_or(&self.endpoint)
    }

    pub fn get_scripts_path(&self) -> &PathBuf {
        self.scripts.path.as_ref().unwrap_or(&self.path)
    }

    pub fn get_scripts_endpoint(&self) -> &str {
        self.scripts.endpoint.as_ref().unwrap_or(&self.endpoint)
    }

    pub fn get_files_path(&self) -> &PathBuf {
        self.files.path.as_ref().unwrap_or(&self.path)
    }

    pub fn get_files_endpoint(&self) -> &str {
        self.files.endpoint.as_ref().unwrap_or(&self.endpoint)
    }

    fn default_index_word() -> String {
        "index".to_string()
    }

    fn default_path() -> PathBuf {
        PathBuf::from(".")
    }

    fn default_endpoint() -> String {
        "/".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FileConfig {
    #[serde(default)]
    suffixes: Vec<String>,
    path: Option<PathBuf>,
    endpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TailwindConfig {
    #[serde(default = "TailwindConfig::default_enable")]
    enable: bool,
    #[serde(default = "TailwindConfig::default_check_rendered")]
    check_rendered: bool,
}

impl Default for TailwindConfig {
    fn default() -> Self {
        Self {
            enable: Self::default_enable(),
            check_rendered: Self::default_check_rendered(),
        }
    }
}

impl TailwindConfig {
    fn default_enable() -> bool {
        true
    }

    fn default_check_rendered() -> bool {
        true
    }
}
