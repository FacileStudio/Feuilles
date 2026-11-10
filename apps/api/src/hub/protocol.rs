//! The binary frames of the collaboration socket. Integers are big-endian and the first byte is
//! the frame type.

use serde::{Deserialize, Serialize};

/// Client to server: one batch from `flushSendQueue()`.
pub const PUSH: u8 = 0x01;
/// Client to server: the selection, as JSON. Not stored.
pub const CURSOR: u8 = 0x02;
/// Client to server: the first frame, naming the browser session.
pub const HELLO: u8 = 0x03;
/// Server to client: the first frame, with the snapshot.
pub const INIT: u8 = 0x10;
/// Server to client: an accepted batch.
pub const UPDATE: u8 = 0x11;
/// Server to client: a refused batch.
pub const REJECT: u8 = 0x12;
/// Server to client: who is here, as JSON.
pub const PRESENCE: u8 = 0x13;

/// A selection: the active cell and the selected range `[r1, c1, r2, c2]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    /// The sheet index.
    pub sheet: u32,
    /// The active row.
    pub row: i32,
    /// The active column.
    pub column: i32,
    /// The selected range.
    pub range: [i32; 4],
}

/// A frame a client sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientFrame {
    /// `PUSH`.
    Push {
        /// The client's own counter, echoed back in `UPDATE` or `REJECT`.
        client_seq: u32,
        /// The batch.
        payload: Vec<u8>,
    },
    /// `CURSOR`.
    Cursor(Cursor),
    /// `HELLO`: a random id, stable for one browser session across its reconnects.
    Hello([u8; 16]),
}

/// Why a client frame did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Empty, too short, or an unknown type: the connection closes with 1002.
    Malformed,
    /// A `CURSOR` whose JSON is not a cursor: the frame is dropped.
    BadCursor,
}

/// Decodes one binary frame from a client.
pub fn decode(frame: &[u8]) -> Result<ClientFrame, FrameError> {
    match frame.split_first() {
        Some((&PUSH, rest)) if rest.len() >= 4 => {
            let (seq, payload) = rest.split_at(4);
            let client_seq = u32::from_be_bytes([seq[0], seq[1], seq[2], seq[3]]);
            Ok(ClientFrame::Push { client_seq, payload: payload.to_vec() })
        }
        Some((&HELLO, rest)) => rest.try_into().map(ClientFrame::Hello).map_err(|_| FrameError::Malformed),
        Some((&CURSOR, rest)) => {
            serde_json::from_slice(rest).map(ClientFrame::Cursor).map_err(|_| FrameError::BadCursor)
        }
        _ => Err(FrameError::Malformed),
    }
}

/// `INIT`: `seq: u64`, `client_id: u32`, `role: u8`, `acked: u32`, `snapshot`. `acked` is the
/// highest `client_seq` accepted from this origin, 0 when none.
pub fn init(seq: u64, client_id: u32, role: u8, acked: u32, snapshot: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(18 + snapshot.len());
    frame.push(INIT);
    frame.extend(seq.to_be_bytes());
    frame.extend(client_id.to_be_bytes());
    frame.push(role);
    frame.extend(acked.to_be_bytes());
    frame.extend(snapshot);
    frame
}

/// `UPDATE`: `seq: u64`, `client_id: u32`, `client_seq: u32`, `payload`.
pub fn update(seq: u64, client_id: u32, client_seq: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(17 + payload.len());
    frame.push(UPDATE);
    frame.extend(seq.to_be_bytes());
    frame.extend(client_id.to_be_bytes());
    frame.extend(client_seq.to_be_bytes());
    frame.extend(payload);
    frame
}

/// `REJECT`: `client_seq: u32`, then the reason in UTF-8.
pub fn reject(client_seq: u32, reason: &str) -> Vec<u8> {
    let mut frame = vec![REJECT];
    frame.extend(client_seq.to_be_bytes());
    frame.extend(reason.as_bytes());
    frame
}

/// `PRESENCE`: the JSON list.
pub fn presence(json: &[u8]) -> Vec<u8> {
    let mut frame = vec![PRESENCE];
    frame.extend(json);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_have_the_documented_layout() {
        assert_eq!(init(2, 7, 2, 5, b"S"), [0x10, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 7, 2, 0, 0, 0, 5, b'S']);
        assert_eq!(update(1, 0, 9, b"P"), [0x11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 9, b'P']);
        assert_eq!(reject(3, "rate_limited")[..5], [0x12, 0, 0, 0, 3]);
        assert_eq!(&reject(3, "rate_limited")[5..], b"rate_limited");
        assert_eq!(presence(b"[]"), [0x13, b'[', b']']);
    }

    #[test]
    fn client_frames_decode_or_say_why_not() {
        let push = decode(&[0x01, 0, 0, 1, 0, 0xAA]).expect("push");
        assert_eq!(push, ClientFrame::Push { client_seq: 256, payload: vec![0xAA] });
        let cursor = br#"{"sheet":0,"row":1,"column":2,"range":[1,2,3,4]}"#;
        let decoded = decode(&[&[CURSOR][..], cursor].concat()).expect("cursor");
        assert_eq!(decoded, ClientFrame::Cursor(Cursor { sheet: 0, row: 1, column: 2, range: [1, 2, 3, 4] }));
        assert_eq!(decode(&[0x02, b'{']), Err(FrameError::BadCursor));
        assert_eq!(decode(&[[0x03].as_slice(), &[9; 16]].concat()), Ok(ClientFrame::Hello([9; 16])));
        assert_eq!(decode(&[0x03, 1, 2]), Err(FrameError::Malformed));
        for bad in [&[][..], &[0x01, 0, 0], &[0x10, 0, 0, 0, 0], &[0x7F]] {
            assert_eq!(decode(bad), Err(FrameError::Malformed), "{bad:?}");
        }
    }
}
