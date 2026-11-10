use std::collections::VecDeque;

use ironcalc::base::UserModel;

use crate::{Downstream, Edit, Stats, Update, apply_edit, from_bytes, new_model};

/// One browser session: a confirmed model, a view model and the pending queue.
pub struct Client {
    /// Connection id the server gave this client.
    pub id: u32,
    /// Applies only batches from the server, in `seq` order.
    pub confirmed: UserModel<'static>,
    /// What the grid draws: `confirmed` plus pending batches.
    pub view: UserModel<'static>,
    /// Local batches not yet echoed back, oldest first.
    pub pending: VecDeque<(u32, Vec<u8>)>,
    /// Last seen server sequence number.
    pub last_seq: u64,
    next_client_seq: u32,
    empty_batch: Vec<u8>,
}

impl Client {
    /// Creates a client from the INIT snapshot.
    pub fn new(id: u32, snapshot: &[u8]) -> Self {
        Client {
            id,
            confirmed: from_bytes(snapshot),
            view: from_bytes(snapshot),
            pending: VecDeque::new(),
            last_seq: 0,
            next_client_seq: 1,
            empty_batch: new_model().flush_send_queue(),
        }
    }

    /// Applies a local edit to `view` and returns the PUSH to send, if the edit produced a batch.
    pub fn local_edit(&mut self, edit: &Edit, stats: &mut Stats) -> Option<(u32, Vec<u8>)> {
        if apply_edit(&mut self.view, edit).is_err() {
            stats.local_errors += 1;
        }
        let batch = self.view.flush_send_queue();
        if batch == self.empty_batch {
            return None;
        }
        let client_seq = self.next_client_seq;
        self.next_client_seq += 1;
        self.pending.push_back((client_seq, batch.clone()));
        Some((client_seq, batch))
    }

    /// Handles one frame from the server.
    pub fn receive(&mut self, frame: Downstream, stats: &mut Stats) {
        match frame {
            Downstream::Update(update) => self.on_update(update, stats),
            Downstream::Reject { client_seq } => {
                self.remove_pending(client_seq);
                self.rebuild_view(stats);
            }
        }
    }

    fn on_update(&mut self, update: Update, stats: &mut Stats) {
        assert_eq!(update.seq, self.last_seq + 1, "updates arrive in seq order");
        self.last_seq = update.seq;
        self.confirmed
            .apply_external_diffs(&update.payload)
            .expect("a batch the server accepted applies on every confirmed model");
        if update.client_id == self.id {
            self.remove_pending(update.client_seq);
        } else if self.pending.is_empty() {
            self.view
                .apply_external_diffs(&update.payload)
                .expect("a batch the server accepted applies on an idle view");
        } else {
            self.rebuild_view(stats);
        }
    }

    fn remove_pending(&mut self, client_seq: u32) {
        let front = self.pending.pop_front().map(|(s, _)| s);
        assert_eq!(front, Some(client_seq), "acks arrive in push order");
    }

    fn rebuild_view(&mut self, stats: &mut Stats) {
        stats.rebuilds += 1;
        self.view = from_bytes(&self.confirmed.to_bytes());
        for (_, batch) in &self.pending {
            if self.view.apply_external_diffs(batch).is_err() {
                stats.replay_errors += 1;
            }
        }
    }
}
