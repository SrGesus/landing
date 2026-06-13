use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Receiver},
    },
    thread::sleep,
    time::Duration,
};

use anyhow::anyhow;
use notify::{EventKind, RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
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
    fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
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
        Ok(config)
    }

    fn update_config(config: &RwLock<Self>, path: impl AsRef<Path>) -> Result<bool, Error> {
        tracing::info!("Reloading config {} ...", path.as_ref().to_string_lossy());
        let new_config = Config::from_file(path)?;
        if new_config != *config.read().unwrap() {
            *config.write().unwrap() = new_config;
            Ok(true)
        } else {
            Ok(false)
        }
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

impl Default for Config {
    fn default() -> Self {
        Self {
            path: Config::default_path(),
            endpoint: Config::default_endpoint(),
            index_word: Config::default_index_word(),
            include: Default::default(),
            templates: Default::default(),
            scripts: Default::default(),
            files: Default::default(),
            tailwind: Default::default(),
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

#[derive(Debug)]
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    path: PathBuf,
    watcher_rx: Mutex<Receiver<Result<notify::Event, notify::Error>>>,
    pub config: Arc<RwLock<Config>>,
}

impl ConfigWatcher {
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let config = Arc::new(RwLock::new(
            Config::from_file(&path)
                .inspect_err(|err| tracing::error!("{}", err))
                .unwrap_or_default(),
        ));

        let (tx, watcher_rx) = mpsc::channel::<Result<notify::Event, notify::Error>>();

        let mut watcher = notify::recommended_watcher(tx).unwrap();

        watcher
            .watch(path.as_ref(), notify::RecursiveMode::NonRecursive)
            .unwrap();
        Ok(ConfigWatcher {
            path: PathBuf::from(path.as_ref()),
            _watcher: watcher,
            watcher_rx: Mutex::new(watcher_rx),
            config,
        })
    }

    pub async fn await_new(self: Arc<Self>) {
        let watcher_rx = self.watcher_rx.lock().unwrap();
        while let Ok(res) = watcher_rx.recv().inspect_err(|e| tracing::error!("{}", e)) {
            match res {
                Ok(event) if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) => {
                    tracing::debug!("Received event: {:?}", event);
                    // Ignore new events for a bit
                    sleep(Duration::from_millis(5));
                    while watcher_rx.try_recv().is_ok() {}
                    match Config::update_config(&self.config, &self.path) {
                        Ok(true) => break,
                        Ok(false) => tracing::warn!("Will NOT update config: Config unchanged."),
                        Err(err) => tracing::error!("Will NOT update config: {}", err),
                    }
                }
                Err(e) => tracing::error!("Watcher error: {}", e),
                _ => (),
            }
        }
    }
}
