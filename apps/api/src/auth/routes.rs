use axum::Router;
use axum::extract::State;
use axum::routing::get;
use porte::Identity;
use porte::session::Manager;
use serde::Serialize;
use tronc::json::Json;

use crate::state::AppState;

/// The signed-in account.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Me {
    /// `porte_users.id`.
    pub id: i64,
    /// The address.
    pub email: String,
    /// The display name.
    pub name: String,
}

/// `GET /auth/session`: the account, or null without a session.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SessionAnswer {
    /// The account, or null.
    pub user: Option<Me>,
}

/// `GET /auth/me` behind the session, `GET /auth/session` open to anyone.
pub fn routes(sessions: &Manager) -> Router<AppState> {
    let me = Router::new().route("/auth/me", get(me)).route_layer(sessions.require_auth());
    let session = Router::new().route("/auth/session", get(session)).route_layer(sessions.optional());
    me.merge(session)
}

async fn account(state: &AppState, identity: &Identity) -> Result<Me, tronc::Error> {
    let row: Option<(String, String)> = sqlx::query_as("SELECT email, name FROM porte_users WHERE id = $1")
        .bind(identity.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| tronc::Error::internal("read the account", e))?;
    let (email, name) = row.ok_or_else(|| tronc::Error::unauthorized("invalid session"))?;
    Ok(Me { id: identity.user_id, email, name })
}

async fn me(State(state): State<AppState>, identity: Identity) -> Result<Json<Me>, tronc::Error> {
    account(&state, &identity).await.map(Json)
}

async fn session(State(state): State<AppState>, identity: Option<Identity>) -> Json<SessionAnswer> {
    let user = match identity {
        Some(identity) => account(&state, &identity).await.ok(),
        None => None,
    };
    Json(SessionAnswer { user })
}
