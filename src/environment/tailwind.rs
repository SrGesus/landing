use std::{
    collections::HashSet, fmt::format, hash::{DefaultHasher, Hasher}, sync::Arc
};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use once_cell::sync::Lazy;
use regex::Regex;
use tailwind_css::TailwindBuilder;
use tracing::debug;

use super::Environment;

#[derive(Debug)]
pub(super) struct Tailwind {
    builder: TailwindBuilder,
    classes: HashSet<String>,
    bundle: String,
    pub etag: String,
    changed: bool, // if true then bundle is out of date
}

static TAILWIND_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"class="([\w\/:\-\s]+)""#).unwrap());

impl Tailwind {
    pub fn new() -> Self {
        Tailwind {
            builder: TailwindBuilder::default(),
            bundle: String::new(),
            classes: HashSet::new(),
            etag: String::new(),
            changed: true,
        }
    }

    pub fn finish(&mut self) -> (String, String) {
        tracing::info!("FInish him");
        let bundle = self.builder.bundle().unwrap();
        let mut hasher = DefaultHasher::default();
        hasher.write(bundle.as_bytes());
        self.bundle = bundle;
        self.etag = format!("\"{:x}\"", hasher.finish());
        self.changed = false;
        (self.bundle.clone(), self.etag.clone())
    }

    pub fn add_content(env: &Environment, content: &str) {
        let classes = TAILWIND_REGEX
            .captures_iter(content)
            .map(|c| {
                let (_, [classes]) = c.extract();
                classes
            })
            .flat_map(|s| s.split_whitespace());

        for class in classes {
            if !env.0.read().unwrap().tailwind.classes.contains(class) {
                let mut guard = env.0.write().unwrap();
                guard.tailwind.classes.insert(class.to_owned());
                if let Ok(_) = guard.tailwind.builder.trace(class, false) {
                    debug!("Found new tailwind class: {}", class);
                    guard.tailwind.changed = true;
                }
            }
        }
    }

    pub fn get_bundle(env: &Environment) -> (String, String) {
        if env.0.read().unwrap().tailwind.changed {
            env.0.write().unwrap().tailwind.finish()
        } else {
            let guard = env.0.read().unwrap();
            (guard.tailwind.bundle.clone(), guard.tailwind.etag.clone())
        }
    }
}

#[axum::debug_handler]
pub async fn get_tailwind(
    State(env): State<Arc<Environment>>,
    headers: HeaderMap,
) -> Response<String> {
    let (bundle, etag) = Tailwind::get_bundle(&env);

    // If none match
    if let Some(client_etag) = headers.get(axum::http::header::IF_NONE_MATCH)
        && *client_etag == *etag
    {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .body("".to_owned())
            .unwrap();
    }

    // If match
    if let Some(client_etag) = headers.get(axum::http::header::IF_MATCH)
        && *client_etag != *etag
    {
        return Response::builder()
            .status(StatusCode::PRECONDITION_FAILED)
            .body("".to_owned())
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("ETag", etag)
        .body(bundle)
        .unwrap()
}
