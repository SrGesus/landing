use std::{
    borrow::Cow,
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use axum::{
    extract::Request,
    response::{Html, IntoResponse, Response},
};
use futures::{FutureExt, future::BoxFuture};
use http::StatusCode;
use minijinja::{Environment, context};
use tokio::fs;

use crate::{config::Config, services::Tailwind};

#[derive(Clone, Debug)]
pub struct Templates {
    environment: Arc<RwLock<Environment<'static>>>,
}

// Result future or err

impl Default for Templates {
    fn default() -> Self {
        Self::new()
    }
}

impl Templates {
    pub fn new() -> Self {
        Templates {
            environment: Arc::new(RwLock::new(Environment::new())),
        }
    }
}

pub trait TemplatesState: Sized + Clone + Send + 'static {
    fn config(&self) -> Config;
    fn tailwind(&self) -> Option<Tailwind>;
    fn templates(&self) -> Templates;

    fn build_templates(
        self,
    ) -> impl std::future::Future<Output = Self> + std::marker::Send + 'static {
        async move {
            let templates_path = self.config().read().get_templates_path().to_path_buf();

            self.handle_dir(templates_path).await
        }
    }

    fn handle_dir(
        self,
        path: PathBuf,
    ) -> impl std::future::Future<Output = Self> + std::marker::Send + 'static {
        async move {
            let mut stack = vec![path];
            let mut handles = vec![];

            while let Some(dir) = stack.pop() {
                let mut dir = fs::read_dir(dir).await.unwrap();

                while let Some(entry) = dir.next_entry().await.unwrap() {
                    let metadata = entry.metadata().await.unwrap();
                    if metadata.is_dir() {
                        stack.push(entry.path());
                    } else if metadata.is_file() {
                        handles.push(tokio::spawn(self.clone().handle_file(entry.path())));
                    }
                }
            }

            for handle in handles {
                handle.await.unwrap();
            }

            self
        }
    }

    fn handle_file(
        self,
        path: PathBuf,
    ) -> impl std::future::Future<Output = ()> + std::marker::Send + 'static {
        async move {
            let file_name = path
                .strip_prefix(self.config().read().get_templates_path())
                .unwrap()
                .to_string_lossy();

            let mut template_names = vec![];
            for suffix in self.config().read().get_templates_suffixes() {
                if let Some(name) = &file_name.strip_suffix(suffix) {
                    template_names.push(name.to_string());
                }
            }

            if template_names.is_empty() {
                return;
            }

            let template_contents = Cow::Owned(fs::read_to_string(&path).await.unwrap());

            // Get all css classes for tailwind
            if let Some(tailwind) = self.tailwind() {
                tailwind.add_content(&template_contents);
            }

            // Load templates
            let templates = self.templates();
            let mut guard = templates.environment.write().unwrap();
            for name in template_names {
                tracing::debug!("Adding template: {}", name);
                if guard.get_template(&name).is_ok() {
                    tracing::warn!("Loaded template \"{}\" more than once.", name);
                }
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

    fn not_found(self, _req: Request) -> Response {
        // TODO
        (StatusCode::NOT_FOUND, Html("404 not found")).into_response()
    }

    fn not_found_future(self, req: Request) -> BoxFuture<'static, Result<Response, Infallible>> {
        async move { Ok(self.not_found(req)) }.boxed()
    }

    fn internal_error(self, _req: Request) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("500 internal server error"),
        )
            .into_response()
    }

    fn render_template(
        self,
        req: Request,
        name: String,
    ) -> BoxFuture<'static, Result<Response, Infallible>> {
        async move {
            match self
                .templates()
                .environment
                .read()
                .unwrap()
                .get_template(&name)
            {
                Ok(template) => Ok(Html(template.render(context! {}).unwrap()).into_response()),
                Err(_) => Ok(self.internal_error(req)),
            }
        }
        .boxed()
    }

    #[allow(clippy::result_large_err)]
    fn try_call_templates(
        self,
        req: Request,
        name: String,
    ) -> Result<BoxFuture<'static, Result<Response, Infallible>>, (Self, Request)> {
        match self
            .templates()
            .environment
            .read()
            .unwrap()
            .get_template(&name)
        {
            Ok(_) => Ok(self.render_template(req, name)),
            Err(_) => Err((self, req)),
        }
    }
}
