use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Init { seq: u64, client_id: u32, role: u8, acked: u32, snapshot: Vec<u8> },
    Update { seq: u64, client_id: u32, client_seq: u32, payload: Vec<u8> },
    Reject { client_seq: u32, reason: String },
    Presence(Value),
    Closed(Option<u16>),
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[..4].try_into().expect("u32"))
}

fn be64(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(bytes[..8].try_into().expect("u64"))
}

pub fn event(frame: &[u8]) -> Event {
    match frame[0] {
        0x10 => Event::Init {
            seq: be64(&frame[1..]),
            client_id: be32(&frame[9..]),
            role: frame[13],
            acked: be32(&frame[14..]),
            snapshot: frame[18..].to_vec(),
        },
        0x11 => Event::Update {
            seq: be64(&frame[1..]),
            client_id: be32(&frame[9..]),
            client_seq: be32(&frame[13..]),
            payload: frame[17..].to_vec(),
        },
        0x12 => {
            Event::Reject { client_seq: be32(&frame[1..]), reason: String::from_utf8_lossy(&frame[5..]).into_owned() }
        }
        0x13 => Event::Presence(serde_json::from_slice(&frame[1..]).expect("presence")),
        other => panic!("unknown frame type {other:#x}"),
    }
}
