use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use crate::error::Error;

mod inner;
mod watcher;

pub use inner::ConfigInner;
use minijinja::{
    Value,
    value::{Enumerator, Object},
};
pub use watcher::ConfigWatcher;

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

impl Object for Config {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "tailwind_href" => Some(Value::from(self.read().get_tailwind_endpoint())),
            _ => None,
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(&["tailwind_href"])
    }
}
