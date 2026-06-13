use std::{
    convert::Infallible,
    path::Path,
    sync::{Arc, RwLock},
};

use axum::{
    Router,
    body::Body,
    extract::Request,
    response::{Html, Response},
};
use futures::future::BoxFuture;
use http::{HeaderMap, Uri};
use tower::{Service, util::BoxCloneSyncService};
use tower_http::services::ServeDir;

use crate::{
    config::{Config, ConfigWatcher},
    tailwind::Tailwind,
    utils::MapIntoResponse,
};

#[derive(Clone, Debug)]
pub struct App {
    tailwind: Tailwind,
    serve_dir: BoxCloneSyncService<Request, Response<Body>, Infallible>,
    config: Arc<RwLock<Config>>,
}

impl App {
    pub(super) async fn build(config: Arc<RwLock<Config>>) -> Self {
        let guard = config.read().unwrap();

        tracing::info!(
            "Static files endpoint {} serving path {}",
            guard.get_files_endpoint(),
            guard.get_files_path().to_string_lossy()
        );
        let serve_dir =
            BoxCloneSyncService::new(MapIntoResponse::new(ServeDir::new(guard.get_files_path())));

        tracing::info!("Serving tailwind at {}", guard.get_tailwind_endpoint());
        let tailwind = Tailwind::new();

        drop(guard);

        App {
            serve_dir,
            tailwind,
            config,
        }
    }

    pub async fn serve(path: impl AsRef<Path>) {
        let config_watcher = Arc::new(
            ConfigWatcher::from_file(path)
                .await
                .expect("Building config"),
        );

        loop {
            let state = Self::build(config_watcher.config.clone()).await;

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
        }
    }

    fn router(self) -> Router {
        Router::new().fallback_service(self)
    }

    pub fn try_call(
        &mut self,
        mut req: Request,
    ) -> BoxFuture<'static, Result<Response, Infallible>> {
        let config = self.config.read().unwrap();

        if config.get_tailwind_enable() && *req.uri() == *config.get_tailwind_endpoint() {
            return self.tailwind.call(req);
        }

        if let Some(asset_uri) = Self::get_assets_uri(&config, req.uri()) {
            *req.uri_mut() = asset_uri;
            // must have 404 service as fallback
            return self.serve_dir.call(req);
        }

        // 404 service
        self.serve_dir.call(req)
    }

    fn get_assets_uri(config: &Config, uri: &Uri) -> Option<Uri> {
        let uri_string = uri.to_string();
        let mut assets_endpoint = config.get_files_endpoint().chars();
        assets_endpoint.next_back();
        tracing::error!("{}", assets_endpoint.as_str());
        uri_string
            .strip_prefix(assets_endpoint.as_str())?
            .parse()
            .ok()
    }
}

async fn handler_wildcard(all_headers: HeaderMap) -> Html<String> {
    Html(format!("{:?}\n", all_headers))
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
        self.clone().try_call(req)
    }
}
