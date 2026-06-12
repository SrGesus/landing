use std::{path::Path, sync::Arc};

use axum::Router;
use tower_http::services::ServeDir;

mod state;

pub use state::AppState;

use crate::config::ConfigWatcher;

pub struct App;
impl App {
    pub async fn serve(path: impl AsRef<Path>) {
        let config_watcher = Arc::new(
            ConfigWatcher::from_file(path)
                .await
                .expect("Building config"),
        );
        let state = AppState::build(config_watcher.config.clone()).await;

        loop {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            tracing::info!(
                "Landing running at http://{}/",
                listener.local_addr().unwrap()
            );

            // Serve axum with clean shutdown when config.toml changes to new valid config
            axum::serve(listener, Self::router(state.clone()))
                .with_graceful_shutdown(config_watcher.clone().await_new())
                .await
                .unwrap();
        }
    }

    fn router(state: AppState) -> Router {
        let config = state.config.read().unwrap();
        let mut router = Router::new();
        tracing::info!("Assets endpoint: {}", config.get_files_endpoint());
        if config.get_files_endpoint() == "/" {
            router = router.fallback_service(ServeDir::new(config.get_files_path()));
        } else {
            router = router.nest_service(
                config.get_files_endpoint(),
                ServeDir::new(config.get_files_path()),
            );
        }
        drop(config);

        router.with_state(state)
    }
}
