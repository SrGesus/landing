use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread::sleep,
    time::Duration,
};

use notify::{EventKind, RecommendedWatcher, Watcher};

use crate::error::Error;
use super::Config;

#[derive(Debug)]
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    path: PathBuf,
    watcher_rx: Mutex<Receiver<Result<notify::Event, notify::Error>>>,
    pub config: Config,
}

impl ConfigWatcher {
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let config = Config::from_file(&path)
            .inspect_err(|err| tracing::error!("{}", err))
            .unwrap_or_default();

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
