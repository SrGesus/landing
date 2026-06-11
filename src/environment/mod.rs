use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::fs::{self};
use tracing::Level;

mod config;
pub mod tailwind;

use self::{config::Config, tailwind::Tailwind};

#[derive(Debug)]
pub struct Environment(pub RwLock<EnvironmentInner>);

#[derive(Debug)]
pub struct EnvironmentInner {
    pub jinja: minijinja::Environment<'static>,
    // pub tailwind: TailwindBuilder,
    pub tailwind: Tailwind,
    pub tailwind_parsed: String,
    pub config: Config,
}

impl Environment {
    pub async fn build() -> Arc<Environment> {
        let span = tracing::span!(Level::INFO, "Environment::build",);

        // Config
        let config_str = fs::read_to_string("./config.toml").await.unwrap();
        let config: Config = toml::from_str(&config_str).unwrap();
        println!("{:#?}", config);

        // Templates
        let _enter = span.enter();
        let mut stack = vec![config.get_templates_path().to_owned()];
        let mut handles = vec![];
        let env = Arc::new(Environment(RwLock::new(EnvironmentInner {
            jinja: minijinja::Environment::new(),
            tailwind: Tailwind::new(),
            tailwind_parsed: String::new(),
            config,
        })));

        while let Some(dir) = stack.pop() {
            let mut dir = fs::read_dir(dir).await.unwrap();

            while let Some(entry) = dir.next_entry().await.unwrap() {
                let metadata = entry.metadata().await.unwrap();
                if metadata.is_dir() {
                    stack.push(entry.path());
                } else if metadata.is_file() {
                    handles.push(tokio::spawn(Self::handle_file(entry.path(), env.clone())));
                }
            }
        }

        for handle in handles {
            handle.await.unwrap();
        }

        env.0.write().unwrap().tailwind.finish();

        env
    }

    async fn handle_file(path: PathBuf, env: Arc<Environment>) {
        let template_contents: String = fs::read_to_string(&path).await.unwrap();

        // Get all css classes for tailwind
        Tailwind::add_content(&env, &template_contents);

        let template_name = path
            .strip_prefix(env.0.read().unwrap().config.get_templates_path())
            .unwrap()
            .to_string_lossy()
            .to_string();

        tracing::debug!("Adding jinja template: {}", template_name);

        // Load templates into jinja
        env.0
            .write()
            .unwrap()
            .jinja
            .add_template_owned(template_name, template_contents)
            .unwrap();
    }
}
