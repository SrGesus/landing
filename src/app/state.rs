use std::sync::{Arc, RwLock};

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct AppState {
    pub(super) config: Arc<RwLock<Config>>,
}

impl AppState {
    pub(super) async fn build(config: Arc<RwLock<Config>>) -> Self {
        AppState { config }
    }

    // This is a std lock so no awaits pls
    pub fn get_config(&self) -> std::sync::RwLockReadGuard<'_, Config> {
        self.config.read().unwrap()
    }
}
