use std::{convert::Infallible, sync::{Arc, RwLock}};

use axum::{extract::Request, response::Response};

use crate::{config::Config, tailwind::Tailwind};

#[derive(Clone, Debug)]
pub struct AppState {
    pub(super) config: Arc<RwLock<Config>>,
    pub(super) tailwind: Tailwind,
}

impl AppState {
    pub(super) async fn build(config: Arc<RwLock<Config>>) -> Self {
        AppState {
            config,
            tailwind: Tailwind::new(),
        }
    }

    // This is a std lock so no awaits pls
    pub fn get_config(&self) -> std::sync::RwLockReadGuard<'_, Config> {
        self.config.read().unwrap()
    }
}


