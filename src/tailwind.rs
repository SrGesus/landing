use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hasher as _},
    sync::{Arc, RwLock},
};

use axum::extract::State;
use http::{HeaderMap, Response, StatusCode};
use once_cell::sync::Lazy;
use regex::Regex;
use tailwind_css::TailwindBuilder;

use crate::app::AppState;

static TAILWIND_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"class="([\w\/:\-\s]+)""#).unwrap());

#[derive(Debug)]
pub(super) struct Tailwind {
    builder: TailwindBuilder,
    classes: HashSet<String>,
    bundle: String,
    pub etag: String,
    changed: bool, // if true then bundle is out of date
}

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

    pub fn add_content(tailwind: Arc<RwLock<Tailwind>>, content: &str) {
        let classes = TAILWIND_REGEX
            .captures_iter(content)
            .map(|c| {
                let (_, [classes]) = c.extract();
                classes
            })
            .flat_map(|s| s.split_whitespace());

        for class in classes {
            if !tailwind.read().unwrap().classes.contains(class) {
                let mut guard = tailwind.write().unwrap();
                guard.classes.insert(class.to_owned());
                if guard.builder.trace(class, false).is_ok() {
                    tracing::debug!("Found new tailwind class: {}", class);
                    guard.changed = true;
                }
            }
        }
    }

    pub fn get_bundle(tailwind: Arc<RwLock<Tailwind>>) -> (String, String) {
        if tailwind.read().unwrap().changed {
            tailwind.write().unwrap().finish()
        } else {
            let tailwind = tailwind.read().unwrap();
            (tailwind.bundle.clone(), tailwind.etag.clone())
        }
    }

    pub async fn call(state: State<AppState>, headers: HeaderMap) -> Response<String> {
        let (bundle, etag) = Tailwind::get_bundle(state.0.tailwind);

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
}
