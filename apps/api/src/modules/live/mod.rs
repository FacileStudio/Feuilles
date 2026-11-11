//! The collaboration socket, and the calls other modules make on open rooms.

mod handshake;
mod rate;
mod router;
mod session;
mod socket;

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use http::Method;
use uuid::Uuid;

use crate::auth::Evict;
use crate::hub::{Command, Hub};
use crate::state::AppState;

pub use router::routes;
pub use socket::codes;

/// The close code that tells a client to reconnect at once: the workbook's state changed, such
/// as going to or out of the trash.
pub const RECONNECT: u16 = 1012;

/// The close code for a workbook that is gone or that the caller lost access to.
pub const POLICY: u16 = 1008;

/// Closes every socket of a workbook whose trash state changed, so each reconnects with the
/// right role.
pub fn reopen(state: &AppState, workbook: Uuid) {
    state.hub.send(Command::Close { workbook, code: RECONNECT });
}

/// Closes every socket of deleted workbooks with 1008.
pub fn close(hub: &Hub, workbooks: impl IntoIterator<Item = Uuid>) {
    workbooks.into_iter().for_each(|workbook| hub.send(Command::Close { workbook, code: POLICY }));
}

/// The directory's eviction hook: a person who lost a space loses its sockets, a deleted
/// workbook loses all of them, and a deleted person loses every socket they hold.
pub fn evictor(hub: Hub) -> Evict {
    Arc::new(move |evictions| {
        for eviction in evictions {
            match (eviction.workbook, eviction.subject) {
                (Some(workbook), Some(subject)) => hub.send(Command::Evict { workbook, subject }),
                (Some(workbook), None) => close(&hub, [workbook]),
                (None, Some(subject)) => hub.send(Command::EvictEverywhere { subject }),
                (None, None) => {}
            }
        }
    })
}

/// Middleware over `/api`: after a logout or a back-channel logout succeeds, every socket checks
/// its session again, so a revoked one closes with 1008 at once instead of at the next ping.
pub async fn on_logout(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let logout = request.method() == Method::POST && request.uri().path().ends_with("logout");
    let response = next.run(request).await;
    if logout && response.status().is_success() {
        state.hub.sessions.changed();
    }
    response
}
