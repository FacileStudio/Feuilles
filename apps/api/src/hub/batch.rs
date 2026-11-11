use tronc::Error;

use super::protocol;
use super::room::Room;
use super::shard::Ctx;
use super::{Answer, Change, Query, Role, Written};
use crate::engine;
use crate::modules::workbooks::history::{self, Batch};

/// The close code when the database refused a batch the model already took.
pub const INTERNAL: u16 = 1011;

/// Cells the server may write before it drops the model's undo history, which nothing on the
/// server ever undoes.
pub const UNDO_CELLS: usize = 20_000;

/// PUSH payloads larger than this pay a full cell count after they apply. So does every 64th
/// batch and every batch that changes the sheets.
pub const COUNT_CELLS_PAST: usize = 64 << 10;

/// The REJECT reason for a batch that takes the workbook past `max_sheets` or `max_cells`.
pub const TOO_LARGE: &str = "too_large";

impl Room {
    /// A batch from a socket: refused to its sender, or stored and broadcast to everyone.
    pub fn push(&mut self, ctx: &Ctx, client_id: u32, client_seq: u32, payload: &[u8]) {
        let Some(conn) = self.conns.get(&client_id) else {
            return;
        };
        let outbox = conn.outbox.clone();
        let refuse = |reason: &str| _ = outbox.send(protocol::reject(client_seq, reason));
        if conn.role == Role::Viewer {
            return refuse("permission_denied");
        }
        if payload.is_empty() || payload == ctx.empty_batch.as_slice() {
            return refuse("invalid_argument");
        }
        if self.acks.get(&conn.origin).is_some_and(|acked| client_seq <= *acked) {
            return;
        }
        let (author, origin) = (Some(conn.member.user_id), Some((conn.origin, client_seq)));
        if let Some(reason) = self.apply_push(ctx, payload) {
            refuse(reason);
            return self.reload(ctx);
        }
        let _ = self.accept(ctx, client_id, Batch { seq: self.seq + 1, author, origin, payload });
    }

    /// Applies a PUSH payload: the REJECT reason when the engine refused it or it took the model
    /// past a cap. The caller then reloads the model.
    fn apply_push(&mut self, ctx: &Ctx, payload: &[u8]) -> Option<&'static str> {
        let sheets = engine::sheet_count(&self.model);
        if self.model.apply_external_diffs(payload).is_err() {
            return Some("invalid_argument");
        }
        let count_cells = sheets != engine::sheet_count(&self.model)
            || payload.len() > COUNT_CELLS_PAST
            || self.seq.is_multiple_of(64);
        self.oversized(ctx, count_cells).then_some(TOO_LARGE)
    }

    /// Stores a batch the model took as the next seq, then broadcasts it, sender included.
    pub fn accept(&mut self, ctx: &Ctx, client_id: u32, batch: Batch<'_>) -> Result<u64, Error> {
        let seq = batch.seq;
        if let Err(error) = ctx.block_on(history::append(&ctx.pool, self.id, &batch)) {
            tracing::error!(workbook = %self.id, error = %error.message, "rooms: a batch could not be stored");
            self.close_all(INTERNAL, "the workbook could not be saved, reconnect");
            return Err(error);
        }
        self.seq = seq;
        let client_seq = batch.origin.map_or(0, |(origin, client_seq)| {
            self.acks.insert(origin, client_seq);
            client_seq
        });
        let frame = protocol::update(seq, client_id, client_seq, batch.payload);
        self.conns.values().for_each(|conn| _ = conn.outbox.send(frame.clone()));
        if seq - self.snapshot_seq >= ctx.limits.snapshot_every {
            self.snapshot(ctx);
        }
        Ok(seq)
    }

    /// Drops the model and rebuilds it from the database: IronCalc may have half-applied a
    /// batch it refused.
    pub fn reload(&mut self, ctx: &Ctx) {
        match ctx.block_on(history::load(&ctx.pool, self.id)) {
            Ok(loaded) => {
                (self.model, self.seq, self.snapshot_seq, self.acks) =
                    (loaded.model, loaded.seq, loaded.snapshot_seq, loaded.acks)
            }
            Err(error) => {
                tracing::error!(workbook = %self.id, error = %error.message, "rooms: reload failed");
                self.close_all(INTERNAL, "the workbook could not be reloaded, reconnect");
            }
        }
    }

    /// Whether the model went past the sheet cap, or past the cell cap when `count_cells`: the
    /// count walks every cell, so a push pays it only when it changed the sheets or was large.
    fn oversized(&self, ctx: &Ctx, count_cells: bool) -> bool {
        engine::sheet_count(&self.model) > ctx.limits.max_sheets
            || (count_cells && engine::cell_count(&self.model) > ctx.limits.max_cells)
    }

    /// Applies an API or MCP write as client 0.
    pub fn write(&mut self, ctx: &Ctx, author: Option<i64>, changes: &[Change]) -> Result<Written, Error> {
        let applied = apply(&mut self.model, changes).and_then(|()| match self.oversized(ctx, true) {
            true => Err(Error::too_large("the workbook would pass its sheet or cell limit")),
            false => Ok(()),
        });
        if let Err(error) = applied {
            self.reload(ctx);
            return Err(error);
        }
        let payload = self.model.flush_send_queue();
        let batch = Batch { seq: self.seq + 1, author, origin: None, payload: &payload };
        let seq = self.accept(ctx, 0, batch)?;
        self.undo_cells += changes.len();
        if self.undo_cells >= UNDO_CELLS {
            self.forget_undo();
        }
        Ok(Written { seq, sheets: engine::sheets(&self.model) })
    }

    /// Answers a read from the current model.
    pub fn read(&self, query: &Query) -> Result<Answer, Error> {
        Ok(match query {
            Query::Range { sheet, area } => Answer::Range(engine::read_area(&self.model, *sheet, *area)?),
            Query::Sheets => Answer::Sheets(engine::sheets(&self.model)),
            Query::Bytes => Answer::Bytes(self.model.to_bytes(), self.language),
        })
    }
}

fn apply(model: &mut engine::Model, changes: &[Change]) -> Result<(), Error> {
    let mut cells = Vec::new();
    for change in changes {
        match change {
            Change::Cell(write) => cells.push(write.clone()),
            Change::AddSheet(name) => {
                engine::set_inputs(model, &std::mem::take(&mut cells))?;
                engine::new_sheet(model, name.as_deref())?;
            }
        }
    }
    engine::set_inputs(model, &cells)
}
