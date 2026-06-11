use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hasher},
    sync::Arc,
};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use once_cell::sync::Lazy;
use regex::Regex;
use tailwind_css::TailwindBuilder;

use super::Environment;

#[derive(Debug)]
pub(super) struct Tailwind {
    builder: TailwindBuilder,
    classes: HashSet<String>,
    pub bundle: String,
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
            changed: false,
        }
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
                    guard.tailwind.changed = true;
                }
            }
        }
    }

    pub fn finish(&mut self) {
        let bundle = self.builder.bundle().unwrap();
        let mut hasher = DefaultHasher::default();
        hasher.write(bundle.as_bytes());
        let hash = hasher.finish();
        self.bundle = bundle;
        self.etag = hash.to_string();
        self.changed = false;
    }
}

#[axum::debug_handler]
pub async fn get_tailwind(
    State(env): State<Arc<Environment>>,
    headers: HeaderMap,
) -> Response<String> {
    if let Some(etag) = headers.get(axum::http::header::IF_NONE_MATCH)
        && *etag == *env.0.read().unwrap().tailwind.etag {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body("".to_owned())
                .unwrap();
        }
    Response::builder()
        .status(StatusCode::OK)
        .header("ETag", &env.0.read().unwrap().tailwind.etag)
        .body(env.0.read().unwrap().tailwind.bundle.clone())
        .unwrap()
}
