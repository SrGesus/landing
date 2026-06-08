use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use tower_http::services::ServeDir;

use crate::environment::Environment;

#[axum::debug_handler]
async fn get_tailwind(state: State<Arc<Environment>>) -> String {
    state.0.0.read().unwrap().tailwind_parsed.clone()
}

pub(crate) fn router(state: Arc<Environment>) -> Router {
    Router::new()
        .route("/assets/tailwind.css", get(get_tailwind))
        .nest_service("/assets/", ServeDir::new("assets"))
        .with_state(state)
}
