use std::sync::Arc;
use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};

use tokio::sync::mpsc;

/// The close code for a connection that cannot keep up with broadcasts.
pub const TRY_AGAIN_LATER: u16 = 1013;

/// What a room hands one connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A binary frame to send, counted against the limit.
    Data(Vec<u8>),
    /// The INIT frame: sent first, even when it alone is larger than the limit, and counted
    /// until the socket takes it, so broadcasts behind it close a socket that does not read.
    Init(Vec<u8>),
    /// Close the socket with this code and reason.
    Close(u16, String),
}

/// The room's end of one connection: frames to send, counted in bytes against a limit.
#[derive(Debug, Clone)]
pub struct Outbox {
    tx: mpsc::UnboundedSender<Frame>,
    queued: Arc<AtomicUsize>,
    closing: Arc<AtomicU16>,
    limit: usize,
}

/// The socket's end: the frames, in order.
#[derive(Debug)]
pub struct Inbox {
    rx: mpsc::UnboundedReceiver<Frame>,
    queued: Arc<AtomicUsize>,
    closing: Arc<AtomicU16>,
}

/// A connected pair with at most `limit` bytes waiting.
pub fn pair(limit: usize) -> (Outbox, Inbox) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (queued, closing) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicU16::new(0)));
    let outbox = Outbox { tx, queued: Arc::clone(&queued), closing: Arc::clone(&closing), limit };
    (outbox, Inbox { rx, queued, closing })
}

impl Outbox {
    /// Queues a frame. Over the byte limit the connection is closed with 1013 instead. Returns
    /// whether the frame was queued.
    pub fn send(&self, frame: Vec<u8>) -> bool {
        if self.closing.load(Ordering::SeqCst) != 0 {
            return false;
        }
        if self.queued.fetch_add(frame.len(), Ordering::SeqCst) + frame.len() > self.limit {
            self.close(TRY_AGAIN_LATER, "too slow to follow the room");
            return false;
        }
        self.tx.send(Frame::Data(frame)).is_ok()
    }

    /// Queues the INIT frame. It always goes, but its bytes count against the limit.
    pub fn init(&self, frame: Vec<u8>) {
        self.queued.fetch_add(frame.len(), Ordering::SeqCst);
        let _ = self.tx.send(Frame::Init(frame));
    }

    /// Closes the connection with `code`. Frames still waiting are dropped.
    pub fn close(&self, code: u16, reason: &str) {
        if self.closing.compare_exchange(0, code, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            let _ = self.tx.send(Frame::Close(code, reason.to_owned()));
        }
    }
}

impl Inbox {
    /// The next frame to act on: a close as soon as one is asked for, even with data waiting.
    pub async fn recv(&mut self) -> Option<Frame> {
        let frame = self.rx.recv().await?;
        if let Frame::Data(bytes) | Frame::Init(bytes) = &frame {
            self.queued.fetch_sub(bytes.len(), Ordering::SeqCst);
        }
        match self.closing.load(Ordering::SeqCst) {
            0 => Some(frame),
            code => Some(Frame::Close(code, close_reason(frame))),
        }
    }
}

fn close_reason(frame: Frame) -> String {
    match frame {
        Frame::Close(_, reason) => reason,
        Frame::Data(_) | Frame::Init(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn going_over_the_limit_closes_with_1013_and_init_counts_until_taken() {
        let (outbox, mut inbox) = pair(10);
        assert!(outbox.send(vec![0; 6]));
        assert!(!outbox.send(vec![0; 6]));
        assert!(!outbox.send(vec![0; 1]));
        assert_eq!(inbox.recv().await, Some(Frame::Close(TRY_AGAIN_LATER, String::new())));
        let (outbox, mut inbox) = pair(10);
        outbox.init(vec![0; 25]);
        assert!(!outbox.send(vec![0; 1]), "a broadcast behind an untaken INIT over the limit closes");
        assert_eq!(inbox.recv().await, Some(Frame::Close(TRY_AGAIN_LATER, String::new())));
        let (outbox, mut inbox) = pair(10);
        outbox.init(vec![0; 25]);
        assert_eq!(inbox.recv().await, Some(Frame::Init(vec![0; 25])));
        assert!(outbox.send(vec![0; 8]), "a taken INIT frees its bytes");
    }

    #[tokio::test]
    async fn frames_arrive_in_order_and_free_their_bytes() {
        let (outbox, mut inbox) = pair(10);
        for _ in 0..5 {
            assert!(outbox.send(vec![1; 8]));
            assert_eq!(inbox.recv().await, Some(Frame::Data(vec![1; 8])));
        }
        outbox.close(1008, "removed");
        outbox.close(1001, "ignored");
        assert_eq!(inbox.recv().await, Some(Frame::Close(1008, "removed".into())));
    }
}
