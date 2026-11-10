//! Collaboration rooms. One room per open workbook holds the authoritative model, orders every
//! batch, stores it, then broadcasts it. A workbook always maps to the same shard thread, so
//! one model is only ever touched by one thread.

mod batch;
mod limits;
mod outbox;
pub mod protocol;
mod room;
mod shard;

use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use sqlx::PgPool;
use tokio::sync::oneshot;
use tronc::Error;
use uuid::Uuid;

pub use limits::Limits;
pub use outbox::{Frame, Inbox, Outbox, TRY_AGAIN_LATER, pair};
pub use protocol::Cursor;

use crate::engine::{Area, Cell, CellWrite, SheetInfo};
pub use crate::modules::workbooks::history::Origin;

/// What a connection may do in its room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Reads, and sees others. Its PUSH is refused.
    Viewer = 1,
    /// Reads and writes.
    Editor = 2,
}

/// Who a connection belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// `porte_users.id`, the author of the batches.
    pub user_id: i64,
    /// Registre's subject, matched by evictions.
    pub subject: Option<String>,
    /// Shown to the others.
    pub name: String,
    /// Shown to the others.
    pub picture: String,
}

/// What a join hands back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Joined {
    /// The connection's id in the room, never 0.
    pub client_id: u32,
}

/// What a read asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// The cells of an area.
    Range {
        /// The sheet index.
        sheet: u32,
        /// The cells.
        area: Area,
    },
    /// The sheets.
    Sheets,
    /// The model's bytes and language, for an export.
    Bytes,
}

/// What a read answers.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// `Query::Range`.
    Range(Vec<Vec<Cell>>),
    /// `Query::Sheets`.
    Sheets(Vec<SheetInfo>),
    /// `Query::Bytes`.
    Bytes(Vec<u8>, &'static str),
}

/// One change a server-side write makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Set a cell input.
    Cell(CellWrite),
    /// Add a sheet at the end, optionally named.
    AddSheet(Option<String>),
}

/// What a write answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The batch's seq.
    pub seq: u64,
    /// The sheets after it.
    pub sheets: Vec<SheetInfo>,
}

/// A reply channel.
pub type Reply<T> = oneshot::Sender<Result<T, Error>>;

/// What a shard does.
#[derive(Debug)]
pub enum Command {
    /// A socket joins, and its first frame is INIT.
    Join { workbook: Uuid, member: Member, role: Role, origin: Origin, outbox: Outbox, reply: Reply<Joined> },
    /// A batch from a socket.
    Push { workbook: Uuid, client_id: u32, client_seq: u32, payload: Vec<u8> },
    /// A selection from a socket.
    Cursor { workbook: Uuid, client_id: u32, cursor: Cursor },
    /// A socket went away.
    Leave { workbook: Uuid, client_id: u32 },
    /// Read the current model.
    Read { workbook: Uuid, query: Query, reply: Reply<Answer> },
    /// Change the model from the API or MCP, as client 0.
    Write { workbook: Uuid, author: Option<i64>, changes: Vec<Change>, reply: Reply<Written> },
    /// `subject` lost access: close their sockets with 1008.
    Evict { workbook: Uuid, subject: String },
    /// Close every socket with `code` and unload the room.
    Close { workbook: Uuid, code: u16 },
    /// Snapshot every room, close every socket with 1001, stop.
    Shutdown,
}

impl Command {
    pub(crate) fn workbook(&self) -> Option<Uuid> {
        match self {
            Command::Join { workbook, .. }
            | Command::Push { workbook, .. }
            | Command::Cursor { workbook, .. }
            | Command::Leave { workbook, .. }
            | Command::Read { workbook, .. }
            | Command::Write { workbook, .. }
            | Command::Evict { workbook, .. }
            | Command::Close { workbook, .. } => Some(*workbook),
            Command::Shutdown => None,
        }
    }
}

/// The shards. Cloning shares them.
#[derive(Clone)]
pub struct Hub {
    shards: Arc<Vec<mpsc::Sender<Command>>>,
    threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
    limits: Limits,
}

impl Hub {
    /// Starts `shards` threads (at least one) with the default limits.
    pub fn start(pool: PgPool, runtime: tokio::runtime::Handle, shards: usize) -> Hub {
        Hub::with_limits(pool, runtime, shards, Limits::default())
    }

    /// [`Hub::start`] with other limits.
    pub fn with_limits(pool: PgPool, runtime: tokio::runtime::Handle, shards: usize, limits: Limits) -> Hub {
        let (senders, threads) =
            (0..shards.max(1)).map(|index| shard::spawn(index, pool.clone(), runtime.clone(), limits.clone())).unzip();
        Hub { shards: Arc::new(senders), threads: Arc::new(Mutex::new(threads)), limits }
    }

    /// `min(4, available_parallelism)`.
    pub fn default_shards() -> usize {
        std::thread::available_parallelism().map_or(1, |n| n.get().min(4))
    }

    /// The limits in force.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Hands `command` to the shard owning its workbook; `Shutdown` goes to every shard.
    pub fn send(&self, command: Command) {
        match command.workbook() {
            Some(id) => {
                let index = usize::try_from(id.as_u128() % self.shards.len() as u128).unwrap_or(0);
                let _ = self.shards[index].send(command);
            }
            None => self.shards.iter().for_each(|shard| _ = shard.send(Command::Shutdown)),
        }
    }

    /// Sends the command `make` builds and waits for its reply.
    pub async fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> Command) -> Result<T, Error> {
        let (reply, answer) = oneshot::channel();
        self.send(make(reply));
        answer.await.map_err(|_| Error::unavailable("the collaboration hub stopped"))?
    }

    /// Snapshots every room, closes every socket with 1001 and waits for the shards. Blocks.
    pub fn shutdown(&self) {
        self.send(Command::Shutdown);
        let threads = std::mem::take(&mut *self.threads.lock().unwrap_or_else(|e| e.into_inner()));
        threads.into_iter().for_each(|thread| _ = thread.join());
    }
}
