use std::time::Duration;

/// The limits a room and its sockets enforce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// The largest PUSH payload. Larger closes the socket with 1009.
    pub max_push: usize,
    /// PUSH frames per second per connection. Over that, REJECT `rate_limited`.
    pub pushes_per_second: usize,
    /// Bytes waiting for one connection. Over that, the socket closes with 1013.
    pub send_buffer: usize,
    /// Silence after which a socket is closed.
    pub silence: Duration,
    /// How often the server pings.
    pub ping: Duration,
    /// How long a room with nobody in it stays loaded.
    pub idle_room: Duration,
    /// Batches between two snapshots.
    pub snapshot_every: u64,
    /// The shortest gap between two PRESENCE frames.
    pub presence_interval: Duration,
    /// The most sheets a workbook may reach. A batch past it is refused `too_large`.
    pub max_sheets: usize,
    /// The most filled cells a workbook may reach. A batch past it is refused `too_large`.
    pub max_cells: usize,
    /// The most sockets one person may hold on one workbook. Past it, a join closes with 1013.
    pub max_user_sockets: usize,
    /// The most sockets one workbook may hold. Past it, a join closes with 1013.
    pub max_room_sockets: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_push: 1 << 20,
            pushes_per_second: 60,
            send_buffer: 8 << 20,
            silence: Duration::from_secs(60),
            ping: Duration::from_secs(25),
            idle_room: Duration::from_secs(60),
            snapshot_every: 256,
            presence_interval: Duration::from_millis(100),
            max_sheets: 200,
            max_cells: 5_000_000,
            max_user_sockets: 8,
            max_room_sockets: 200,
        }
    }
}
