use std::{
    borrow::Cow,
    convert::Infallible,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        mpsc::{self, Receiver},
    },
    thread::sleep,
    time::Duration,
};

use axum::{
    extract::Request,
    response::{Html, IntoResponse, Response},
};
use futures::{FutureExt, future::BoxFuture};
use http::StatusCode;
use minijinja::{Environment, context};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::fs;

use crate::{config::Config, services::Tailwind};

pub struct TemplatesWatcher {
    _watcher: RecommendedWatcher,
    pub templates: Templates,
}

impl TemplatesWatcher {
    pub fn new<S: TemplatesState>(state: S) -> Self {
        let (tx, watcher_rx) = mpsc::channel::<Result<notify::Event, notify::Error>>();

        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(
                state.config().read().get_templates_path(),
                RecursiveMode::Recursive,
            )
            .unwrap();

        let templates = Templates::new();

        Self::watcher_task(state, watcher_rx);

        Self {
            _watcher: watcher,
            templates,
        }
    }

    fn watcher_task<S: TemplatesState>(
        state: S,
        watcher_rx: Receiver<Result<notify::Event, notify::Error>>,
    ) {
        tokio::spawn(async move {
            // Ignore new events for a bit
            sleep(Duration::from_millis(5));
            while watcher_rx.try_recv().is_ok() {}

            while let Ok(res) = watcher_rx.recv() {
                match res {
                    Ok(event)
                        if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) =>
                    {
                        tracing::debug!("Received event: {:?}", event);
                        // Ignore new events for a bit
                        sleep(Duration::from_millis(5));
                        while watcher_rx.try_recv().is_ok() {}
                        // Add template
                        // TODO
                        tracing::info!("Templates {:?} modified", event.paths);
                        for path in event.paths {
                            state.clone().handle_file(path, false).await;
                        }
                    }
                    Err(e) => tracing::error!("Watcher error: {}", e),
                    _ => (),
                }
            }
        });
    }
}

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
                if let Ok(mut dir) = fs::read_dir(dir).await {
                    while let Ok(Some(entry)) = dir.next_entry().await {
                        match entry.metadata().await {
                            Ok(metadata) if metadata.is_dir() => stack.push(entry.path()),
                            Ok(metadata) if metadata.is_file() => handles
                                .push(tokio::spawn(self.clone().handle_file(entry.path(), true))),
                            _ => (),
                        }
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
        build: bool,
    ) -> impl std::future::Future<Output = ()> + std::marker::Send + 'static {
        async move {
            if !tokio::fs::metadata(&path).await.is_ok_and(|f| f.is_file()) {
                return;
            }

            let file_name = format!(
                "/{}",
                path.strip_prefix(self.config().read().get_templates_path())
                    .unwrap()
                    .to_string_lossy()
            );

            let mut template_names = vec![file_name.to_string()];
            for suffix in self.config().read().get_templates_suffixes() {
                if let Some(name) = &file_name.strip_suffix(suffix) {
                    template_names.push(name.to_string());
                }
            }

            if template_names.len() <= 1 {
                return;
            }

            if let Ok(template_contents) = fs::read_to_string(&path).await {
                let template_contents = Cow::Owned(template_contents);

                // Get all css classes for tailwind
                if let Some(tailwind) = self.tailwind() {
                    tailwind.add_content(&template_contents);
                }

                // Load templates
                let templates = self.templates();
                let mut guard = templates.environment.write().unwrap();
                for name in template_names {
                    tracing::debug!("Adding template: \"{}\"", name);
                    if guard.get_template(&name).is_ok() && build {
                        tracing::warn!("Template \"{}\" loaded more than once.", name);
                    }
                    if let Err(err) = guard.add_template_owned(name, template_contents.clone()) {
                        tracing::error!(
                            "Could not parse template \"{}\": {}",
                            path.to_string_lossy(),
                            err
                        );
                        return;
                    }
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
