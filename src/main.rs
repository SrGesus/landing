use std::{
    os::linux::raw::stat,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use axum::{Router, extract::State, routing::get};
use itertools::Itertools;
use regex::Regex;
use tailwind_css::TailwindBuilder;
use tokio::fs::{self};
use tracing_subscriber::fmt;

mod environment;
mod router;

use environment::Environment;

use crate::environment::EnvironmentInner;

#[axum::debug_handler]
async fn get_tailwind(state: State<Arc<Environment>>) -> String {
    state.0.0.read().unwrap().tailwind_parsed.clone()
}

#[tokio::main]
async fn main() {
    // build our application with a single route

    fmt()
        // .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("{:?}", Environment::build("./templates").await);
    let env = Environment::build("./templates").await;

    let app = Router::new()
        .route("/assets/tailwind.css", get(get_tailwind))
        .with_state(env);

    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
