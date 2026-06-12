use std::{
    path::Path,
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Receiver},
    },
    thread::sleep,
    time::Duration,
};

use axum::{Router, serve::Listener};
use notify::{Event, EventKind, Watcher};
use tower_http::services::ServeDir;
use tracing_subscriber::fmt;

use crate::{config::Config, error::Error};

const CONFIG_PATH: &str = "config.toml";

pub mod config;
pub mod error;

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<Config>>,
}

impl AppState {
    async fn build() -> Self {
        let config = Config::from_file(CONFIG_PATH)
            .map_err(|e| Error::config(CONFIG_PATH, e))
            .expect("Building config");

        let config = Arc::new(RwLock::new(config));

        AppState { config }
    }

    fn update_config(&self) -> Result<(), Error> {
        tracing::info!("Reloading config {} ...", CONFIG_PATH);
        let config = Config::from_file(CONFIG_PATH).map_err(|e| Error::config(CONFIG_PATH, e))?;
        *self.config.write().unwrap() = config;
        Ok(())
    }
}

struct App;
impl App {
    async fn serve() {
        let state = AppState::build().await;
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        let rx = Arc::new(Mutex::new(rx));

        let mut config_watcher = notify::recommended_watcher(tx).unwrap();
        config_watcher
            .watch(Path::new(CONFIG_PATH), notify::RecursiveMode::NonRecursive)
            .unwrap();

        loop {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            tracing::info!(
                "Landing running at http://{}/",
                listener.local_addr().unwrap()
            );

            let rx = rx.clone();
            let state = state.clone();

            // Serve axum with clean shutdown when config.toml changes to new valid config
            axum::serve(listener, Self::router(state.clone()))
                .with_graceful_shutdown(
                    tokio::spawn(async { Self::new_config(state, rx) })
                        .await
                        .unwrap(),
                )
                .await
                .unwrap();
        }
    }

    async fn new_config(
        state: AppState,
        rx: Arc<Mutex<Receiver<Result<notify::Event, notify::Error>>>>,
    ) {
        let rx = rx.lock().unwrap();
        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) => {
                    // Ignore events for a bit
                    sleep(Duration::from_millis(5));
                    while rx.try_recv().is_ok() {}
                    if let Err(err) = state.update_config() {
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

#[tokio::main]
async fn main() {
    // build our application with a single route

    fmt()
        // .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    App::serve().await;
}
