use axum::Router;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use http::request::Parts;
use porte::Identity;
use porte::session::Manager;
use tronc::{Code, Error};
use uuid::Uuid;

use super::{handshake, socket};
use crate::hub::{Member, Role};
use crate::modules::workbooks::service::{Access, caller, parse_id, require};
use crate::state::AppState;

/// `GET /workbooks/{id}/ws`. The session is optional here: a refused upgrade is completed and
/// closed with 1008, because a browser sees an HTTP 401 or 404 on an upgrade as a network drop.
pub fn routes(sessions: &Manager) -> Router<AppState> {
    Router::new().route("/workbooks/{id}/ws", get(connect)).route_layer(sessions.optional())
}

async fn connect(
    State(state): State<AppState>,
    identity: Option<Identity>,
    Path(id): Path<String>,
    parts: Parts,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Result<Response, Error> {
    state.kits.sessions.check_origin(&parts)?;
    let decided = decide(&state, identity.as_ref(), &id).await;
    let upgrade = match upgrade {
        Ok(upgrade) => upgrade,
        Err(_) => {
            return Err(decided.err().unwrap_or_else(|| Error::invalid("this route only answers a WebSocket upgrade")));
        }
    };
    let reason = match decided {
        Ok((workbook, member, role)) => {
            let (hub, limits) = (state.hub.clone(), state.hub.limits().clone());
            let upgrade = upgrade.max_message_size(limits.max_push * 4).max_frame_size(limits.max_push * 4);
            return Ok(upgrade.on_upgrade(move |ws| socket::run(ws, hub, workbook, member, role)));
        }
        Err(error) if error.code == Code::Unauthenticated => "unauthenticated",
        Err(error) if error.code == Code::NotFound => "not_found",
        Err(error) => return Err(error),
    };
    Ok(upgrade.on_upgrade(move |ws| handshake::refuse(ws, reason)))
}

async fn decide(state: &AppState, identity: Option<&Identity>, id: &str) -> Result<(Uuid, Member, Role), Error> {
    let identity = identity.ok_or_else(|| Error::unauthorized("missing auth"))?;
    let (row, access) = require(state, identity, parse_id(id)?, Access::Viewer).await?;
    let role = match access >= Access::Editor && row.trashed_at.is_none() {
        true => Role::Editor,
        false => Role::Viewer,
    };
    Ok((row.id, member(state, identity).await?, role))
}

async fn member(state: &AppState, identity: &Identity) -> Result<Member, Error> {
    let caller = caller(state, identity).await?;
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT u.name, u.email, COALESCE((SELECT picture FROM space_members
             WHERE subject = $2 AND picture <> '' LIMIT 1), '')
         FROM porte_users u WHERE u.id = $1",
    )
    .bind(identity.user_id)
    .bind(caller.subject.as_deref().unwrap_or_default())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::internal("read the member", e))?;
    let (name, email, picture) = row.unwrap_or_default();
    let name = if name.trim().is_empty() { email.split('@').next().unwrap_or_default().to_owned() } else { name };
    Ok(Member { user_id: identity.user_id, subject: caller.subject, name, picture })
}
