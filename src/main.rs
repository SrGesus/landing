use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{Router, routing::get};
use futures::{FutureExt, future::join_all};
use itertools::Itertools;
use minijinja::Environment;
use regex::{Match, Regex};
use std::fmt::Write;
use tailwind_css::TailwindBuilder;
use tokio::{
    fs::{self, metadata},
    join,
    sync::Semaphore,
};
use tracing::info;
use tracing_subscriber::{EnvFilter, filter::Directive, fmt};

#[derive(Debug)]
struct Templates {
    templates: Vec<(String,)>,
}

async fn handle_file<'a>(
    path: PathBuf,
    jinja: Arc<Mutex<Environment<'a>>>,
    tailwind: Arc<Mutex<TailwindBuilder>>,
) {
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
        tracing::debug!(
            "We got classes: {:?}",
            tailwind.lock().unwrap().inline(&classes).unwrap()
        );
        tailwind.lock().unwrap().inline(&classes).unwrap();
    }

    // Load templates into jinja
    jinja
        .lock()
        .unwrap()
        .add_template_owned(
            path.components()
                .map(|f| f.as_os_str().to_string_lossy())
                .join("/"),
            template_contents,
        )
        .unwrap();
}

async fn build_environment(templates_path: &str) -> String {
    let mut stack = vec![PathBuf::from(templates_path)];
    let mut handles = vec![];
    let jinja = Arc::new(Mutex::new(Environment::new()));
    let tailwind = Arc::new(Mutex::new(TailwindBuilder::default()));

    while let Some(dir) = stack.pop() {
        let mut dir = fs::read_dir(dir).await.unwrap();

        while let Some(entry) = dir.next_entry().await.unwrap() {
            let metadata = entry.metadata().await.unwrap();
            if metadata.is_dir() {
                stack.push(entry.path());
            } else if metadata.is_file() {
                handles.push(tokio::spawn(handle_file(
                    entry.path(),
                    jinja.clone(),
                    tailwind.clone(),
                )));
            }
        }
    }

    for handle in handles {
        handle.await.unwrap();
    }

    Arc::try_unwrap(jinja).unwrap().into_inner().unwrap();
    Arc::try_unwrap(tailwind)
        .unwrap()
        .into_inner()
        .unwrap()
        .bundle()
        .unwrap()
}

#[tokio::main]
async fn main() {
    // build our application with a single route

    fmt()
        // .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("{:?}", build_environment("./templates").await);
    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // // run our app with hyper, listening globally on port 3000
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // axum::serve(listener, app).await.unwrap();
}
