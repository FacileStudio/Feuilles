use axum::response::IntoResponse;
use serde::Serialize;
use tronc::json::Json;

use crate::engine::ENGINE_VERSION;

/// The release, stamped at build time from `apps/client/package.json`, or `dev`.
pub const VERSION: &str = match option_env!("APP_VERSION") {
    Some(version) => version,
    None => "dev",
};

/// `GET /version` answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct VersionAnswer {
    /// The release, such as `v0.1.0`.
    pub version: &'static str,
    /// The IronCalc version. The browser's `@ironcalc/wasm` must equal it.
    pub engine: &'static str,
}

/// `GET /version`.
pub async fn handler() -> impl IntoResponse {
    Json(VersionAnswer { version: VERSION, engine: ENGINE_VERSION })
}
