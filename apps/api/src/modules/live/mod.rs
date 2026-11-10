//! The collaboration socket, and the calls other modules make on open rooms.

mod handshake;
mod rate;
mod router;
mod socket;

use std::sync::Arc;

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

/// The directory's eviction hook: a person who lost a space loses its sockets, and a deleted
/// workbook loses all of them.
pub fn evictor(hub: Hub) -> Evict {
    Arc::new(move |evictions| {
        for eviction in evictions {
            match eviction.subject {
                Some(subject) => hub.send(Command::Evict { workbook: eviction.workbook, subject }),
                None => close(&hub, [eviction.workbook]),
            }
        }
    })
}
