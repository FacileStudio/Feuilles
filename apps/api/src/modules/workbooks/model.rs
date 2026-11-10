//! The current state of a workbook's cells, for the routes that read or write them.

use tronc::Error;
use uuid::Uuid;

use super::history;
use super::types::CellWrite;
use crate::engine::{self, Area, Cell, Model, SheetInfo};
use crate::state::AppState;

/// The most cells one write may set.
pub const MAX_WRITES: usize = 5000;

/// The most cells one read may return.
pub const MAX_READ_CELLS: i64 = 10_000;

async fn current(state: &AppState, id: Uuid) -> Result<Model, Error> {
    history::load(&state.pool, id).await.map(|loaded| loaded.model)
}

/// The sheets as they are now.
pub async fn sheets(state: &AppState, id: Uuid) -> Result<Vec<SheetInfo>, Error> {
    current(state, id).await.map(|model| engine::sheets(&model))
}

/// The model as it is now, for an export.
pub async fn snapshot(state: &AppState, id: Uuid) -> Result<Model, Error> {
    current(state, id).await
}

/// The cells of `area`.
pub async fn read(state: &AppState, id: Uuid, sheet: u32, area: Area) -> Result<Vec<Vec<Cell>>, Error> {
    if area.cells() > MAX_READ_CELLS {
        return Err(Error::invalid(format!("a read returns at most {MAX_READ_CELLS} cells")));
    }
    engine::read_area(&current(state, id).await?, sheet, area)
}

/// Applies `writes` as one batch and returns its seq.
pub async fn write(state: &AppState, id: Uuid, author: i64, writes: &[CellWrite]) -> Result<u64, Error> {
    check_writes(writes)?;
    let mut loaded = history::load(&state.pool, id).await?;
    let payload = engine::apply_writes(&mut loaded.model, writes)?;
    let seq = loaded.seq + 1;
    history::append(&state.pool, id, seq, Some(author), &payload).await?;
    Ok(seq)
}

/// Refuses an empty or oversized write.
pub fn check_writes(writes: &[CellWrite]) -> Result<(), Error> {
    match writes.len() {
        0 => Err(Error::invalid("writes must hold at least one cell")),
        n if n > MAX_WRITES => Err(Error::invalid(format!("writes holds at most {MAX_WRITES} cells"))),
        _ => Ok(()),
    }
}
