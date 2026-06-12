use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock, mpsc::{self, Receiver}},
    thread::sleep,
    time::Duration,
};

use notify::{EventKind, RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct ConfigInner {
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
}

#[derive(Debug)]
pub  struct Config {
    path: PathBuf,
    _watcher: RecommendedWatcher,
    rx: Arc<Mutex<Receiver<Result<notify::Event, notify::Error>>>>,
    inner: ConfigInner,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel::<Result<notify::Event, notify::Error>>();
        let mut watcher = notify::recommended_watcher(tx).unwrap();

        watcher
            .watch(path.as_ref(), notify::RecursiveMode::NonRecursive)
            .unwrap();

        Ok(Config {
            path: PathBuf::from(path.as_ref()),
            inner: ConfigInner::from_file(&path).map_err(|e| Error::config(path, e))?,
            _watcher: watcher,
            rx: Arc::new(Mutex::new(rx)),
        })
    }

    fn update_config(config: &RwLock<Self>) -> Result<(), Error> {
        let path = config.read().unwrap().path.clone();
        tracing::info!("Reloading config {} ...", path.to_string_lossy());
        *config.write().unwrap() = Config::from_file(path)?;
        Ok(())
    }

    pub async fn await_new(
        config: Arc<RwLock<Config>>,
    ) {
        let rx = config.read().unwrap().rx.clone();
        let rx = rx.lock().unwrap();
        while let Ok(res) = rx.recv().inspect_err(|e| tracing::error!("{}", e)) {
            match res {
                Ok(event) if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) => {
                    tracing::debug!("Received event: {:?}", event);
                    // Ignore new events for a bit
                    sleep(Duration::from_millis(5));
                    while rx.try_recv().is_ok() {}
                    if let Err(err) = Config::update_config(&config) {
                        tracing::error!("Error reloading config: {}", err);
                    } else {
                        break;
                    }
                }
                Err(e) => tracing::error!("Watcher error: {}", e),
                _ => (),
            }
        }
    }

    pub fn get_index_word(&self) -> &str {
        &self.inner.index_word
    }

    pub fn get_include(&self) -> &Option<String> {
        &self.inner.include
    }

    pub fn get_templates_path(&self) -> &PathBuf {
        self.inner.templates.path.as_ref().unwrap_or(&self.inner.path)
    }

    pub fn get_templates_endpoint(&self) -> &str {
        self.inner.templates.endpoint.as_ref().unwrap_or(&self.inner.endpoint)
    }

    pub fn get_scripts_path(&self) -> &PathBuf {
        self.inner.scripts.path.as_ref().unwrap_or(&self.inner.path)
    }

    pub fn get_scripts_endpoint(&self) -> &str {
        self.inner.scripts.endpoint.as_ref().unwrap_or(&self.inner.endpoint)
    }

    pub fn get_files_path(&self) -> &PathBuf {
        self.inner.files.path.as_ref().unwrap_or(&self.inner.path)
    }

    pub fn get_files_endpoint(&self) -> &str {
        self.inner.files.endpoint.as_ref().unwrap_or(&self.inner.endpoint)
    }
}

impl ConfigInner {
    fn from_file(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let config_str = std::fs::read_to_string(path.as_ref())?;
        Ok(toml::from_str(&config_str)?)
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
