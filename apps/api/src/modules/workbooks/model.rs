//! The current state of a workbook's cells, read and written through its room, so a write
//! from the API reaches every open browser.

use tronc::Error;
use uuid::Uuid;

use super::types::CellWrite;
use crate::engine::{self, Area, Cell, Model, SheetInfo};
use crate::hub::{Answer, Change, Command, Query, Written};
use crate::state::AppState;

/// The most cells one write may set.
pub const MAX_WRITES: usize = 5000;

/// The most cells one read may return.
pub const MAX_READ_CELLS: i64 = 10_000;

async fn ask(state: &AppState, workbook: Uuid, query: Query) -> Result<Answer, Error> {
    state.hub.ask(|reply| Command::Read { workbook, query, reply }).await
}

fn unexpected() -> Error {
    Error::internal("read a room", "the room answered another query")
}

/// The sheets as they are now.
pub async fn sheets(state: &AppState, id: Uuid) -> Result<Vec<SheetInfo>, Error> {
    match ask(state, id, Query::Sheets).await? {
        Answer::Sheets(sheets) => Ok(sheets),
        _ => Err(unexpected()),
    }
}

/// A copy of the model as it is now, for an export.
pub async fn snapshot(state: &AppState, id: Uuid) -> Result<Model, Error> {
    match ask(state, id, Query::Bytes).await? {
        Answer::Bytes(bytes, language) => engine::load(&bytes, language),
        _ => Err(unexpected()),
    }
}

/// The cells of `area`.
pub async fn read(state: &AppState, id: Uuid, sheet: u32, area: Area) -> Result<Vec<Vec<Cell>>, Error> {
    if area.cells() > MAX_READ_CELLS {
        return Err(Error::invalid(format!("a read returns at most {MAX_READ_CELLS} cells")));
    }
    match ask(state, id, Query::Range { sheet, area }).await? {
        Answer::Range(rows) => Ok(rows),
        _ => Err(unexpected()),
    }
}

/// Applies `changes` as one batch from the server, client 0.
pub async fn change(state: &AppState, workbook: Uuid, author: i64, changes: Vec<Change>) -> Result<Written, Error> {
    let author = Some(author);
    state.hub.ask(|reply| Command::Write { workbook, author, changes, reply }).await
}

/// Applies `writes` as one batch and returns its seq.
pub async fn write(state: &AppState, id: Uuid, author: i64, writes: &[CellWrite]) -> Result<u64, Error> {
    check_writes(writes)?;
    let changes = writes.iter().cloned().map(Change::Cell).collect();
    change(state, id, author, changes).await.map(|written| written.seq)
}

/// Refuses an empty or oversized write.
pub fn check_writes(writes: &[CellWrite]) -> Result<(), Error> {
    match writes.len() {
        0 => Err(Error::invalid("writes must hold at least one cell")),
        n if n > MAX_WRITES => Err(Error::invalid(format!("writes holds at most {MAX_WRITES} cells"))),
        _ => Ok(()),
    }
}
