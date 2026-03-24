use axum::{response::Json, routing::get, Router};
use serde_json::Value;

static OPENAPI_SPEC: &str = include_str!("openapi_spec.json");

pub fn openapi_spec() -> Value {
    serde_json::from_str(OPENAPI_SPEC).expect("openapi_spec.json is valid JSON")
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/openapi.json", get(|| async { Json(openapi_spec()) }))
}
