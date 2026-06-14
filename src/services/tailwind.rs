use std::{
    collections::HashSet,
    convert::Infallible,
    hash::{DefaultHasher, Hasher as _},
    sync::{Arc, RwLock},
};

use axum::{body::Body, extract::Request, response::Response};
use futures::{FutureExt, future::BoxFuture};
use http::{StatusCode, header};
use once_cell::sync::Lazy;
use regex::Regex;
use tailwind_css::TailwindBuilder;
use tower::Service;

static TAILWIND_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"class="([\w\/:\-\s]+)""#).unwrap());

#[derive(Debug)]
struct TailwindInner {
    builder: TailwindBuilder,
    classes: HashSet<String>,
    bundle: String,
    pub etag: String,
    changed: bool, // if true then bundle is out of date
}
#[derive(Debug, Clone)]
pub struct Tailwind(Arc<RwLock<TailwindInner>>);

impl TailwindInner {
    fn finish(&mut self) -> String {
        if self.changed {
            let bundle = self.builder.bundle().unwrap();
            let mut hasher = DefaultHasher::default();
            hasher.write(bundle.as_bytes());
            self.bundle = bundle;
            self.etag = format!("\"{:x}\"", hasher.finish());
            self.changed = false;
        }
        self.etag.clone()
    }
}

impl Default for Tailwind {
    fn default() -> Self {
        Self::new()
    }
}

impl Tailwind {
    pub fn new() -> Self {
        Tailwind(Arc::new(RwLock::new(TailwindInner {
            builder: TailwindBuilder::default(),
            bundle: String::new(),
            classes: HashSet::new(),
            etag: String::new(),
            changed: true,
        })))
    }

    pub fn add_content(self, content: &str) {
        let Tailwind(tailwind) = self;
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

    pub fn get_etag(&self) -> String {
        let Tailwind(tailwind) = self;
        if tailwind.read().unwrap().changed {
            tailwind.write().unwrap().finish()
        } else {
            tailwind.read().unwrap().etag.clone()
        }
    }

    pub fn get_bundle(&self) -> (String, String) {
        let Tailwind(tailwind) = self;
        // Important to guarantee the etag corresponds to the bundle
        if tailwind.read().unwrap().changed {
            let mut tailwind_write = tailwind.write().unwrap();
            (tailwind_write.finish(), tailwind_write.bundle.clone())
        } else {
            let tailwind_read = tailwind.read().unwrap();
            (tailwind_read.etag.clone(), tailwind_read.bundle.clone())
        }
    }

    pub async fn try_call(self, req: Request) -> Result<Response, Infallible> {
        let etag = self.get_etag();
        let headers = req.headers();

        // If none match
        if let Some(client_etag) = headers.get(axum::http::header::IF_NONE_MATCH)
            && *client_etag == *etag
        {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())
                .unwrap());
        }

        // If match
        if let Some(client_etag) = headers.get(axum::http::header::IF_MATCH)
            && *client_etag != *etag
        {
            return Ok(Response::builder()
                .status(StatusCode::PRECONDITION_FAILED)
                .body(Body::empty())
                .unwrap());
        }

        let (etag, bundle) = self.get_bundle();
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::ETAG, etag)
            .header(header::CONTENT_TYPE, "text/css")
            .body(Body::from(bundle))
            .unwrap())
    }
}

impl Service<Request> for Tailwind {
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.clone().try_call(req).boxed()
    }
}
