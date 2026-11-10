use ironcalc::base::UserModel;

use crate::from_bytes;

/// A batch accepted by the server, as broadcast in an UPDATE frame.
#[derive(Debug, Clone)]
pub struct Update {
    /// Server sequence number after this batch.
    pub seq: u64,
    /// Sender of the batch.
    pub client_id: u32,
    /// Sender's sequence number for the batch.
    pub client_seq: u32,
    /// Opaque `flush_send_queue` bytes.
    pub payload: Vec<u8>,
}

/// A frame from the server to one client.
#[derive(Debug, Clone)]
pub enum Downstream {
    /// An accepted batch.
    Update(Update),
    /// A refused batch.
    Reject {
        /// Sender's sequence number for the refused batch.
        client_seq: u32,
    },
}

/// Counters a run reports, to show which paths the random cases exercised.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Local edits the view model refused.
    pub local_errors: u64,
    /// Batches the server accepted.
    pub accepted: u64,
    /// Batches the server rejected.
    pub rejected: u64,
    /// Rejected batches that had already changed the server model before failing.
    pub partial_rejects: u64,
    /// View rebuilds after a concurrent remote batch or a reject.
    pub rebuilds: u64,
    /// Pending batches that failed to replay on a rebuilt view.
    pub replay_errors: u64,
}

impl Stats {
    /// Adds another run's counters to these.
    pub fn add(&mut self, other: &Stats) {
        self.local_errors += other.local_errors;
        self.accepted += other.accepted;
        self.rejected += other.rejected;
        self.partial_rejects += other.partial_rejects;
        self.rebuilds += other.rebuilds;
        self.replay_errors += other.replay_errors;
    }
}

/// The authoritative room: one model and a sequence number.
pub struct Server {
    /// The authoritative model.
    pub model: UserModel<'static>,
    /// Count of accepted batches.
    pub seq: u64,
}

impl Server {
    /// Creates a server from the shared starting snapshot.
    pub fn new(snapshot: &[u8]) -> Self {
        Server {
            model: from_bytes(snapshot),
            seq: 0,
        }
    }

    /// Applies one PUSH. On error the model is restored, so a rejected batch changes nothing.
    pub fn push(
        &mut self,
        client_id: u32,
        client_seq: u32,
        payload: Vec<u8>,
        stats: &mut Stats,
    ) -> Result<Update, String> {
        let before = self.model.to_bytes();
        if let Err(e) = self.model.apply_external_diffs(&payload) {
            stats.rejected += 1;
            if self.model.to_bytes() != before {
                stats.partial_rejects += 1;
            }
            self.model = from_bytes(&before);
            return Err(e);
        }
        self.seq += 1;
        stats.accepted += 1;
        Ok(Update {
            seq: self.seq,
            client_id,
            client_seq,
            payload,
        })
    }
}
