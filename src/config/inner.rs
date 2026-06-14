use std::path::{Path, PathBuf};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ConfigInner {
    #[serde(default = "ConfigInner::default_path")]
    path: PathBuf,
    #[serde(default = "ConfigInner::default_endpoint")]
    endpoint: String,
    #[serde(default = "ConfigInner::default_index_word")]
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
    #[serde(skip)]
    tailwind_endpoint: String,
}

impl ConfigInner {
    pub(super) fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::_from_file(&path).map_err(|e| Error::config(path, e))
    }

    fn _from_file(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let config_str = std::fs::read_to_string(path.as_ref())?;
        let mut config: Self = toml::from_str(&config_str)?;
        Self::validate_endpoint(&mut config.endpoint);
        Self::validate_endpoint_o(&mut config.templates.endpoint);
        Self::validate_endpoint_o(&mut config.scripts.endpoint);
        Self::validate_endpoint_o(&mut config.files.endpoint);
        Self::validate_path(&mut config.path)?;
        Self::validate_path_o(&mut config.templates.path)?;
        Self::validate_path_o(&mut config.scripts.path)?;
        Self::validate_path_o(&mut config.files.path)?;
        config.tailwind_endpoint = format!("{}tailwind.css", config.get_files_endpoint());
        let mut templates_index_suffixes = vec![];
        for suffix in config.get_templates_suffixes() {
            templates_index_suffixes.push(format!("/{}{}", config.get_index_word(), suffix));
        }
        config.templates.suffixes.extend(templates_index_suffixes);
        let mut scripts_index_suffixes = vec![];
        for suffix in config.get_scripts_suffixes() {
            scripts_index_suffixes.push(format!("/{}{}", config.get_index_word(), suffix));
        }
        config.scripts.suffixes.extend(scripts_index_suffixes);
        Ok(config)
    }

    pub fn get_files_uri(&self, uri: &http::Uri) -> Option<http::Uri> {
        let uri_string = uri.to_string();
        let mut files_endpoint = self.get_files_endpoint().chars();
        files_endpoint.next_back();
        uri_string
            .strip_prefix(files_endpoint.as_str())?
            .parse()
            .ok()
    }

    pub fn get_template_name(&self, uri: &http::Uri) -> Option<String> {
        // TODO: fix this mess, cloning too many strings
        let mut template_name = uri.path().to_string();
        Self::validate_endpoint(&mut template_name);
        let mut path = template_name
            .strip_prefix(self.get_templates_endpoint())?
            .to_string();
        if path.pop().is_some() {
            path.insert(0, '/');
        }
        Some(path)
    }

    fn validate_path(path: &mut PathBuf) -> Result<(), anyhow::Error> {
        Self::_validate_path(path)
            .map_err(|err| anyhow!("Invalid path \"{}\": {}", path.to_string_lossy(), err))
    }

    fn _validate_path(path: &mut PathBuf) -> Result<(), anyhow::Error> {
        let path_canon = path.canonicalize()?;
        path_canon.read_dir()?;
        *path = path_canon;
        Ok(())
    }

    fn validate_path_o(path: &mut Option<PathBuf>) -> Result<(), anyhow::Error> {
        if let Some(path) = path {
            Self::validate_path(path)
        } else {
            Ok(())
        }
    }

    fn validate_endpoint(endpoint: &mut String) {
        // Remove sequential /
        let mut i = 0;
        let bytes = unsafe { endpoint.as_mut_vec() };
        for idx in 0..bytes.len() {
            if bytes[idx] == b'/' && i > 0 && bytes[i - 1] == b'/' {
                continue;
            }
            bytes[i] = bytes[idx];
            i += 1;
        }
        bytes.truncate(i);
        // Make sure endpoint ends and starts and ends with /
        if !endpoint.starts_with('/') {
            endpoint.insert(0, '/');
        }
        if !endpoint.ends_with('/') {
            endpoint.push('/');
        }
    }

    fn validate_endpoint_o(endpoint: &mut Option<String>) {
        if let Some(endpoint) = endpoint {
            Self::validate_endpoint(endpoint);
        }
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

    pub fn get_templates_suffixes(&self) -> &Vec<std::string::String> {
        &self.templates.suffixes
    }

    pub fn get_scripts_path(&self) -> &PathBuf {
        self.scripts.path.as_ref().unwrap_or(&self.path)
    }

    pub fn get_scripts_endpoint(&self) -> &str {
        self.scripts.endpoint.as_ref().unwrap_or(&self.endpoint)
    }

    pub fn get_scripts_suffixes(&self) -> &Vec<std::string::String> {
        &self.scripts.suffixes
    }

    pub fn get_files_path(&self) -> &PathBuf {
        self.files.path.as_ref().unwrap_or(&self.path)
    }

    pub fn get_files_endpoint(&self) -> &str {
        self.files.endpoint.as_ref().unwrap_or(&self.endpoint)
    }

    pub fn get_tailwind_enable(&self) -> bool {
        self.tailwind.enable
    }

    pub fn get_tailwind_check_rendered(&self) -> bool {
        self.tailwind.check_rendered
    }

    pub fn get_tailwind_endpoint(&self) -> &str {
        &self.tailwind_endpoint
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

impl Default for ConfigInner {
    fn default() -> Self {
        Self {
            path: Self::default_path(),
            endpoint: Self::default_endpoint(),
            index_word: Self::default_index_word(),
            include: Default::default(),
            templates: Default::default(),
            scripts: Default::default(),
            files: Default::default(),
            tailwind: Default::default(),
            tailwind_endpoint: format!("{}tailwind.css", Self::default_path().to_string_lossy()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct FileConfig {
    #[serde(default)]
    suffixes: Vec<String>,
    path: Option<PathBuf>,
    endpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
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
