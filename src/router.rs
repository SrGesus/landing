use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::get,
};
use minijinja::context;
use tower_http::services::ServeDir;

use crate::environment::{Environment, tailwind::get_tailwind};

#[axum::debug_handler]
async fn get_template(
    State(state): State<Arc<Environment>>,
    Path(path): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let guard = state.0.read().unwrap();

    let mut template_option = None;

    tracing::debug!("Looking for template {}", path);

    for suffix in ["", ".html", "index.html", "/index.html"] {
        if let Ok(t) = guard.jinja.get_template(&format!("{path}{suffix}")) {
            template_option = Some(t);
            break;
        }
    }

    let template = template_option.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Html(template.render(context! {}).unwrap()))
}

pub(crate) fn router(state: Arc<Environment>) -> Router {
    Router::new()
        .route("/assets/tailwind.css", get(get_tailwind))
        .nest_service("/assets/", ServeDir::new("assets"))
        .route("/{*template}", get(get_template))
        .with_state(state)
}
