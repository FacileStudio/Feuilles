use sqlx::PgPool;
use tokio::sync::broadcast;

/// The database the sockets check their sessions in, and the signal that one may have ended.
#[derive(Clone)]
pub struct Sessions {
    /// Where `porte_sessions` lives.
    pub pool: PgPool,
    changed: broadcast::Sender<()>,
}

impl Sessions {
    /// Over `pool`, with nobody listening yet.
    pub fn new(pool: PgPool) -> Sessions {
        Sessions { pool, changed: broadcast::channel(16).0 }
    }

    /// Tells every socket to check its session again now: someone logged out.
    pub fn changed(&self) {
        let _ = self.changed.send(());
    }

    /// Hears [`Sessions::changed`].
    pub fn watch(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }
}
