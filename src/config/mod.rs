use std::{
    path::{Path},
    sync::{
        Arc, RwLock,
    },
};


use crate::error::Error;

mod watcher;
mod inner;

pub use watcher::ConfigWatcher;
pub use inner::ConfigInner;

#[derive(Clone, Debug, Default)]
pub struct Config(Arc<RwLock<ConfigInner>>);

impl Config {
    fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Config(Arc::new(RwLock::new(ConfigInner::from_file(path)?))))
    }

    fn update_config(&self, path: impl AsRef<Path>) -> Result<bool, Error> {
        tracing::info!("Reloading config {} ...", path.as_ref().to_string_lossy());
        let new_config = ConfigInner::from_file(path)?;
        if new_config != *self.0.read().unwrap() {
            *self.0.write().unwrap() = new_config;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, ConfigInner> {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, ConfigInner> {
        self.0.write().unwrap()
    }
}
