use axum::Router;
use porte::mcpauth::require_own_session;
use porte::session::Manager;
use serde::Serialize;
use tronc::mcp::{Action, PendingItem, PendingPage};

use crate::state::AppState;

/// One entry of `GET /api/mcp/pending`: tronc's item plus the agent's stored arguments, which the
/// page shows and reads the workbook from.
#[derive(Serialize)]
pub struct Pending {
    #[serde(flatten)]
    item: PendingItem,
    target: String,
}

/// `GET /mcp/pending`, `POST /mcp/pending/{id}/confirm|dismiss`, behind the session. Only a
/// browser login's own cookie answers: a bearer, a CLI, device or named API token, even sent as
/// the cookie, or a delegated agent gets 403, so an agent never confirms its own request.
pub fn routes(state: &AppState, sessions: &Manager) -> Router<AppState> {
    let page =
        PendingPage::new(state.confirmations.clone(), "feuilles-web", |parts: http::request::Parts| async move {
            require_own_session(&parts).map(|identity| identity.user_id.to_string())
        });
    let page = page.describe(|item, action: Action| async move { Pending { item, target: action.target } });
    page.routes().route_layer(sessions.require_auth())
}
