use tracing_subscriber::{EnvFilter, fmt};

use crate::app::App;

pub mod app;
pub mod config;
pub mod error;

#[tokio::main]
async fn main() {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    App::serve().await;
}
