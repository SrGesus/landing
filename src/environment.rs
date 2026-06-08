use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use itertools::Itertools;
use regex::Regex;
use tailwind_css::TailwindBuilder;
use tokio::fs::{self};
use tracing::Level;

#[derive(Debug)]
pub struct Environment(pub RwLock<EnvironmentInner>);

#[derive(Debug)]
pub struct EnvironmentInner {
    pub jinja: minijinja::Environment<'static>,
    pub templates_path: PathBuf,
    pub tailwind: TailwindBuilder,
    pub tailwind_parsed: String,
}

impl Environment {
    pub async fn build(templates_path: PathBuf) -> Arc<Environment> {
        let span = tracing::span!(
            Level::INFO,
            "Environment::build",
            templates_path = templates_path.to_string_lossy().to_string()
        );
        let _enter = span.enter();
        let mut stack = vec![templates_path.clone()];
        let mut handles = vec![];
        let env = Arc::new(Environment(RwLock::new(EnvironmentInner {
            jinja: minijinja::Environment::new(),
            tailwind: TailwindBuilder::default(),
            tailwind_parsed: String::new(),
            templates_path,
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

        {
            let mut guard = env.0.write().unwrap();
            guard.tailwind_parsed = guard.tailwind.bundle().unwrap();
        }

        env
    }

    async fn handle_file(path: PathBuf, env: Arc<Environment>) {
        let template_contents: String = fs::read_to_string(&path).await.unwrap();
        let re: regex::Regex = Regex::new(r#"class="([\w\/:\-\s]+)""#).unwrap();

        // Get all css classes for tailwind
        let classes = re
            .captures_iter(&template_contents)
            .map(|c| {
                let (_, [classes]) = c.extract();
                classes
            })
            .join(" ");
        if !classes.is_empty() {
            let mut e = env.0.write().unwrap();

            for c in classes.split_whitespace() {
                match e.tailwind.trace(c, false).ok() {
                    Some(_) => tracing::debug!("Seen tailwind class: {}", c),
                    None => tracing::debug!("Seen non-tailwind class: {}", c),
                }
            }
        }

        let template_name = path
            .strip_prefix(&env.0.read().unwrap().templates_path)
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
