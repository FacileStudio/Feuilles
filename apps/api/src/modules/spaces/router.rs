use axum::Router;
use axum::extract::State;
use axum::routing::get;
use porte::Identity;
use porte::session::Manager;
use tronc::json::Json;

use super::service;
use super::types::SpaceList;
use crate::state::AppState;

/// `GET /spaces`, behind the session.
pub fn routes(sessions: &Manager) -> Router<AppState> {
    Router::new().route("/spaces", get(list)).route_layer(sessions.require_auth())
}

async fn list(State(state): State<AppState>, identity: Identity) -> Result<Json<SpaceList>, tronc::Error> {
    Ok(Json(SpaceList { spaces: service::mine(&state, &identity).await? }))
}
