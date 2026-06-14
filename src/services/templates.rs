use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use minijinja::Environment;
use tokio::fs;

use crate::{config::Config, services::Tailwind};

pub struct Templates {
    config: Config,
    environment: Arc<RwLock<Environment<'static>>>,
    tailwind: Tailwind,
}

// Result future or err

impl Templates {
    pub fn new(config: Config, tailwind: Tailwind) -> Self {
        

        Templates {
            config,
            environment: Arc::new(RwLock::new(Environment::new())),
            tailwind,
        }
    }

    async fn handle_file(self, path: PathBuf) {
        let config = self.config.read();
        let file_name = path.file_name().unwrap().to_string_lossy();

        let mut template_names = vec![];
        for suffix in config.get_templates_suffixes() {
            if let Some(name) = &file_name.strip_suffix(suffix) {
                template_names.push(name.to_string());
            }
        }

        if template_names.is_empty() {
            return;
        }

        let template_contents = Cow::Owned(fs::read_to_string(&path).await.unwrap());

        // Get all css classes for tailwind
        if config.get_tailwind_enable() {
            self.tailwind.add_content(&template_contents);
        }

        let mut guard = self.environment.write().unwrap();
        for name in template_names {
            if let Err(err) = guard.add_template_owned(name, template_contents.clone()) {
                tracing::error!(
                    "Could not parse template {}: {}",
                    path.to_string_lossy(),
                    err
                );
            }
        }
    }
}
