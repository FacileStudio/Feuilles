use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tronc::Error;
use uuid::Uuid;

use super::room::{Conn, POLICY, Room};
use super::{Command, Limits};
use crate::engine;

/// The close code on shutdown.
pub const GOING_AWAY: u16 = 1001;

const TICK: Duration = Duration::from_millis(50);

/// What every room of a shard shares.
pub struct Ctx {
    pub(super) pool: PgPool,
    pub(super) limits: Limits,
    pub(super) empty_batch: Vec<u8>,
    runtime: tokio::runtime::Handle,
}

impl Ctx {
    /// Runs database work from the shard thread.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

struct Shard {
    ctx: Ctx,
    rooms: HashMap<Uuid, Room>,
    ticked: Instant,
}

/// Starts one shard thread.
pub fn spawn(
    index: usize,
    pool: PgPool,
    runtime: tokio::runtime::Handle,
    limits: Limits,
) -> (mpsc::Sender<Command>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let ctx = Ctx { pool, limits, empty_batch: engine::empty_batch(), runtime };
    let shard = Shard { ctx, rooms: HashMap::new(), ticked: Instant::now() };
    let thread = std::thread::Builder::new()
        .name(format!("feuilles-room-{index}"))
        .spawn(move || shard.run(&rx))
        .unwrap_or_else(|error| panic!("rooms: cannot start shard {index}: {error}"));
    (tx, thread)
}

impl Shard {
    fn run(mut self, rx: &mpsc::Receiver<Command>) {
        loop {
            match rx.recv_timeout(TICK) {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => return self.shutdown(),
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.housekeeping(Instant::now());
        }
    }

    fn handle(&mut self, command: Command) {
        let (ctx, rooms) = (&self.ctx, &mut self.rooms);
        match command {
            Command::Join { workbook, seat, reply } => {
                let joined = room(ctx, rooms, workbook).and_then(|room| room.join(&ctx.limits, seat));
                let _ = reply.send(joined);
            }
            Command::Read { workbook, query, reply } => {
                let _ = reply.send(room(ctx, rooms, workbook).and_then(|room| room.read(&query)));
            }
            Command::Write { workbook, author, changes, reply } => {
                let _ = reply.send(room(ctx, rooms, workbook).and_then(|room| room.write(ctx, author, &changes)));
            }
            Command::EvictEverywhere { subject } => {
                let gone = |_: u32, conn: &Conn| conn.member.subject.as_deref() == Some(subject.as_str());
                rooms.values_mut().for_each(|room| room.remove(ctx, gone, Some(POLICY)));
            }
            command => {
                if let Some(room) = command.workbook().and_then(|id| rooms.get_mut(&id)) {
                    loaded(ctx, room, command);
                }
            }
        }
        self.rooms.retain(|_, room| !room.broken);
    }

    fn housekeeping(&mut self, now: Instant) {
        if now.duration_since(self.ticked) < TICK {
            return;
        }
        self.ticked = now;
        let (ctx, idle) = (&self.ctx, self.ctx.limits.idle_room);
        for room in self.rooms.values_mut() {
            room.flush_presence(now, ctx.limits.presence_interval);
            if room.empty_since.is_some_and(|since| now.duration_since(since) >= idle) {
                room.snapshot(ctx);
                room.broken = true;
            }
        }
        self.rooms.retain(|_, room| !room.broken);
    }

    fn shutdown(&mut self) {
        for room in self.rooms.values_mut() {
            room.snapshot(&self.ctx);
            room.close_all(GOING_AWAY, "the server is restarting, reconnect");
        }
        self.rooms.clear();
    }
}

fn room<'a>(ctx: &Ctx, rooms: &'a mut HashMap<Uuid, Room>, id: Uuid) -> Result<&'a mut Room, Error> {
    match rooms.entry(id) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => Ok(entry.insert(Room::load(ctx, id)?)),
    }
}

fn loaded(ctx: &Ctx, room: &mut Room, command: Command) {
    match command {
        Command::Push { client_id, client_seq, payload, .. } => room.push(ctx, client_id, client_seq, &payload),
        Command::Cursor { client_id, cursor, .. } => room.cursor(client_id, cursor),
        Command::Leave { client_id, .. } => room.remove(ctx, |id, _| id == client_id, None),
        Command::Evict { subject, .. } => {
            room.remove(ctx, |_, conn| conn.member.subject.as_deref() == Some(&subject), Some(POLICY));
        }
        Command::Close { code, .. } => room.close_all(code, "the workbook changed, reconnect"),
        _ => {}
    }
}
