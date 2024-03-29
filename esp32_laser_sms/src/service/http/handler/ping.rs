use axum::{routing::get, Router};

#[axum::debug_handler]
async fn ping() -> &'static str {
  "pong"
}

pub fn build_router() -> Router {
  Router::new().route("/", get(ping))
}
