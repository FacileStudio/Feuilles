use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, post};
use porte::Identity;
use porte::session::Manager;
use serde::Serialize;
use tronc::json::Json;
use tronc::mcp::{Action, Caller, Confirmations};
use tronc::{Error, Result};

use crate::state::AppState;

#[derive(Serialize)]
struct Pending {
    #[serde(flatten)]
    action: Action,
    state: &'static str,
}

#[derive(Serialize)]
struct Confirmed {
    status: &'static str,
}

fn person(state: &AppState, who: &Identity, headers: &HeaderMap) -> Result<(Confirmations, Caller)> {
    let browser = who.actor.is_empty() && who.session_id != 0 && !headers.contains_key(header::AUTHORIZATION);
    if !browser {
        return Err(Error::forbidden("confirm agent actions from your own browser session in Feuilles"));
    }
    let confirmations = state.confirmations.clone().ok_or_else(|| Error::not_found("MCP is off"))?;
    Ok((confirmations, Caller { user_id: who.user_id.to_string(), ..Caller::default() }))
}

async fn list(State(state): State<AppState>, who: Identity, headers: HeaderMap) -> Result<Json<Vec<Pending>>> {
    let (confirmations, caller) = person(&state, &who, &headers)?;
    let mut out = Vec::new();
    for action in confirmations.open(&caller.user_id).await? {
        let state = confirmations.state(&action).await;
        out.push(Pending { action, state });
    }
    Ok(Json(out))
}

async fn confirm(
    State(state): State<AppState>,
    who: Identity,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Confirmed>> {
    let (confirmations, caller) = person(&state, &who, &headers)?;
    Ok(Json(Confirmed { status: confirmations.confirm(&caller, &id).await?.status }))
}

async fn dismiss(
    State(state): State<AppState>,
    who: Identity,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let (confirmations, caller) = person(&state, &who, &headers)?;
    confirmations.dismiss(&caller, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /mcp/actions`, `POST /mcp/actions/{id}/confirm|dismiss`, behind the session. Only the
/// person's own browser session answers: a bearer token, an API session or a delegated agent
/// gets 403, so an agent never confirms its own request.
pub fn routes(sessions: &Manager) -> Router<AppState> {
    Router::new()
        .route("/mcp/actions", get(list))
        .route("/mcp/actions/{id}/confirm", post(confirm))
        .route("/mcp/actions/{id}/dismiss", post(dismiss))
        .route_layer(sessions.require_auth())
}
