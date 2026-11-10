use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use serde::Serialize;
use tronc::Error;
use uuid::Uuid;

use super::protocol::{self, Cursor};
use super::shard::Ctx;
use super::{Joined, Member, Origin, Outbox, Role};
use crate::engine::Model;
use crate::modules::workbooks::history;

/// The close code for someone who lost access.
pub const POLICY: u16 = 1008;

/// One socket in a room.
pub struct Conn {
    pub(super) member: Member,
    pub(super) origin: Origin,
    pub(super) role: Role,
    pub(super) outbox: Outbox,
    cursor: Option<Cursor>,
}

/// One open workbook: the authoritative model and the sockets following it.
pub struct Room {
    pub(super) id: Uuid,
    pub(super) model: Model,
    pub(super) language: &'static str,
    pub(super) seq: u64,
    pub(super) snapshot_seq: u64,
    pub(super) conns: BTreeMap<u32, Conn>,
    pub(super) empty_since: Option<Instant>,
    pub(super) broken: bool,
    pub(super) acks: HashMap<Origin, u32>,
    next_client: u32,
    presence_dirty: bool,
    presence_sent: Option<Instant>,
}

#[derive(Serialize)]
struct Presence<'a> {
    client_id: u32,
    name: &'a str,
    picture: &'a str,
    cursor: Option<Cursor>,
}

impl Room {
    /// Rebuilds the room from the database.
    pub fn load(ctx: &Ctx, id: Uuid) -> Result<Room, Error> {
        let loaded = ctx.block_on(history::load(&ctx.pool, id))?;
        Ok(Room {
            id,
            model: loaded.model,
            language: loaded.language,
            seq: loaded.seq,
            snapshot_seq: loaded.snapshot_seq,
            conns: BTreeMap::new(),
            empty_since: Some(Instant::now()),
            broken: false,
            acks: loaded.acks,
            next_client: 0,
            presence_dirty: false,
            presence_sent: None,
        })
    }

    /// Adds a socket and queues its INIT frame.
    pub fn join(&mut self, member: Member, role: Role, origin: Origin, outbox: Outbox) -> Joined {
        self.next_client = self.next_client.checked_add(1).unwrap_or(1);
        let client_id = self.next_client;
        let acked = self.acks.get(&origin).copied().unwrap_or(0);
        outbox.init(protocol::init(self.seq, client_id, role as u8, acked, &self.model.to_bytes()));
        self.conns.insert(client_id, Conn { member, origin, role, outbox, cursor: None });
        self.empty_since = None;
        self.presence_dirty = true;
        Joined { client_id }
    }

    /// Records a socket's selection.
    pub fn cursor(&mut self, client_id: u32, cursor: Cursor) {
        if let Some(conn) = self.conns.get_mut(&client_id) {
            conn.cursor = Some(cursor);
            self.presence_dirty = true;
        }
    }

    /// Removes the sockets matching `gone`, closing them with `code` when given.
    pub fn remove(&mut self, ctx: &Ctx, gone: impl Fn(u32, &Conn) -> bool, code: Option<u16>) {
        let ids: Vec<u32> = self.conns.iter().filter(|(id, conn)| gone(**id, conn)).map(|(id, _)| *id).collect();
        for id in ids {
            if let (Some(conn), Some(code)) = (self.conns.remove(&id), code) {
                conn.outbox.close(code, "you no longer have access to this workbook");
            }
            self.presence_dirty = true;
        }
        if self.conns.is_empty() && self.empty_since.is_none() {
            self.snapshot(ctx);
            self.empty_since = Some(Instant::now());
        }
    }

    /// Closes every socket with `code` and marks the room for unloading.
    pub fn close_all(&mut self, code: u16, reason: &str) {
        std::mem::take(&mut self.conns).into_values().for_each(|conn| conn.outbox.close(code, reason));
        self.broken = true;
    }

    /// Sends PRESENCE when it changed, at most once per `interval`.
    pub fn flush_presence(&mut self, now: Instant, interval: std::time::Duration) {
        if !self.presence_dirty || self.presence_sent.is_some_and(|sent| now.duration_since(sent) < interval) {
            return;
        }
        let list: Vec<Presence> = self
            .conns
            .iter()
            .map(|(id, c)| Presence {
                client_id: *id,
                name: &c.member.name,
                picture: &c.member.picture,
                cursor: c.cursor,
            })
            .collect();
        let frame = protocol::presence(&serde_json::to_vec(&list).unwrap_or_default());
        self.conns.values().for_each(|conn| _ = conn.outbox.send(frame.clone()));
        (self.presence_dirty, self.presence_sent) = (false, Some(now));
    }

    /// Writes the snapshot when batches arrived since the last one. A failure is logged: the
    /// update log still holds every batch.
    pub fn snapshot(&mut self, ctx: &Ctx) {
        if self.seq == self.snapshot_seq {
            return;
        }
        match ctx.block_on(history::snapshot(&ctx.pool, self.id, &self.model.to_bytes(), self.seq)) {
            Ok(()) => self.snapshot_seq = self.seq,
            Err(error) => tracing::warn!(workbook = %self.id, error = %error.message, "rooms: snapshot failed"),
        }
    }
}
