use std::{convert::Infallible, path::Path, sync::Arc};

use axum::{Router, body::Body, extract::Request, response::Response};
use futures::future::BoxFuture;
use tower::{Service, util::BoxCloneSyncService};
use tower_http::services::ServeDir;

use crate::{
    config::{Config, ConfigWatcher},
    services::{Tailwind, Templates, TemplatesState, TemplatesWatcher},
    utils::MapIntoResponse,
};

#[derive(Clone, Debug)]
pub struct App {
    config: Config,
    tailwind: Tailwind,
    serve_dir: BoxCloneSyncService<Request, Response<Body>, Infallible>,
    templates: Templates,
}

impl App {
    pub(super) async fn build(config: Config) -> Self {
        // Clippy keeps complaining about the guard even if i drop if before the await
        // so i had to put it in a block
        let (serve_dir, tailwind, templates) = {
            let guard = config.read();

            tracing::info!(
                "Static files serving {} from {}",
                guard.get_files_endpoint(),
                guard.get_files_path().to_string_lossy()
            );
            let serve_dir = BoxCloneSyncService::new(MapIntoResponse::new(ServeDir::new(
                guard.get_files_path(),
            )));

            tracing::info!("Tailwind serving {}", guard.get_tailwind_endpoint());
            let tailwind = Tailwind::new();

            tracing::info!(
                "Templates serving {} from {}",
                guard.get_templates_endpoint(),
                guard.get_templates_path().to_string_lossy()
            );
            let templates = Templates::new();
            (serve_dir, tailwind, templates)
        };

        App {
            config,
            serve_dir,
            tailwind,
            templates,
        }
        .build_templates()
        .await
    }

    pub async fn watch(&self) -> (TemplatesWatcher,) {
        let templates_watcher = TemplatesWatcher::new(self.clone());

        (templates_watcher,)
    }

    pub async fn serve(path: impl AsRef<Path>) {
        let config_watcher = Arc::new(
            ConfigWatcher::from_file(path)
                .await
                .expect("Building config"),
        );

        loop {
            let state = Self::build(config_watcher.config.clone()).await;
            let watchers = state.watch().await;

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            tracing::info!(
                "Landing running at http://{}/",
                listener.local_addr().unwrap()
            );

            // Serve axum with clean shutdown when config.toml changes to new valid config
            axum::serve(listener, Self::router(state.clone()))
                .with_graceful_shutdown(config_watcher.clone().await_new())
                .await
                .unwrap();
            drop(watchers)
        }
    }

    fn router(self) -> Router {
        Router::new().fallback_service(self)
    }

    fn app_call(mut self, mut req: Request) -> BoxFuture<'static, Result<Response, Infallible>> {
        // Tailwind route
        if *req.uri() == *self.config.read().get_tailwind_endpoint()
            && let Some(mut tailwind) = self.tailwind()
        {
            return tailwind.call(req);
        }

        // tracing::info!(
        //     "We got {} {}",
        //     req.uri(),
        //     self.config.read().get_templates_endpoint()
        // );

        // Templates route
        let template_name = self.config.read().get_template_name(req.uri());
        if let Some(template_name) = template_name {
            tracing::debug!("Looking for template \"{}\"", template_name);
            (self, req) = match self.try_call_templates(req, template_name) {
                Ok(future) => return future,
                Err((req, state)) => (req, state),
            };
        }

        // Assets Route
        if let Some(asset_uri) = self.config.read().get_files_uri(req.uri()) {
            *req.uri_mut() = asset_uri;
            // must have 404 service as fallback
            return self.serve_dir.call(req);
        }

        // 404 service
        self.not_found_future(req)
    }
}

impl Service<Request> for App {
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.serve_dir.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.clone().app_call(req)
    }
}

impl TemplatesState for App {
    fn config(&self) -> Config {
        self.config.clone()
    }

    fn tailwind(&self) -> Option<Tailwind> {
        if self.config.read().get_tailwind_enable() {
            Some(self.tailwind.clone())
        } else {
            None
        }
    }

    fn templates(&self) -> Templates {
        self.templates.clone()
    }
}
