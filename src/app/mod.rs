use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{self},
    },
};

use axum::Router;
use notify::{Event, Watcher};
use tower_http::services::ServeDir;

mod state;

pub use state::AppState;

use crate::config::Config;

const CONFIG_PATH: &str = "config.toml";

pub struct App;
impl App {
    pub async fn serve() {
        let state = AppState::build(CONFIG_PATH).await;
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        let rx = Arc::new(Mutex::new(rx));

        let mut watcher = notify::recommended_watcher(tx.clone()).unwrap();

        watcher
            .watch(Path::new(CONFIG_PATH), notify::RecursiveMode::NonRecursive)
            .unwrap();

        loop {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            tracing::info!(
                "Landing running at http://{}/",
                listener.local_addr().unwrap()
            );

            // Serve axum with clean shutdown when config.toml changes to new valid config
            axum::serve(listener, Self::router(state.clone()))
                .with_graceful_shutdown(Config::await_new(state.config.clone(), rx.clone()))
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
