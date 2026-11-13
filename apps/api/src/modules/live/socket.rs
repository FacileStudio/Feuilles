use std::time::Instant;

use axum::extract::ws::{Message, WebSocket};
use uuid::Uuid;

use super::handshake::{close, hello, too_long};
use super::rate::Rate;
use super::session;
use crate::hub::protocol::{self, ClientFrame, FrameError};
use crate::hub::{Command, Frame, Hub, Joined, Limits, Member, Outbox, Role, Seat};

/// Close codes the socket sends itself.
pub mod codes {
    /// Silence, or shutdown.
    pub const GOING_AWAY: u16 = 1001;
    /// A frame that does not decode.
    pub const PROTOCOL: u16 = 1002;
    /// A text frame.
    pub const UNSUPPORTED: u16 = 1003;
    /// A PUSH over the size limit.
    pub const TOO_BIG: u16 = 1009;
    /// No session, no access, or no such workbook: the client stops retrying.
    pub const POLICY: u16 = 1008;
    /// The room could not be joined.
    pub const INTERNAL: u16 = 1011;
}

pub(super) enum Flow {
    Continue,
    Close(u16, String),
    Stop,
}

struct Connection {
    hub: Hub,
    limits: Limits,
    workbook: Uuid,
    client_id: u32,
    role: Role,
    outbox: Outbox,
    rate: Rate,
    heard: Instant,
    session_id: i64,
}

/// Runs one collaboration socket until either side closes it. The first client frame must be
/// HELLO.
pub async fn run(mut socket: WebSocket, hub: Hub, workbook: Uuid, member: Member, role: Role) {
    let limits = hub.limits().clone();
    let origin = match hello(&mut socket, limits.silence).await {
        Ok(origin) => origin,
        Err(Flow::Close(code, reason)) => return close(&mut socket, code, &reason).await,
        Err(_) => return,
    };
    let (outbox, mut inbox) = crate::hub::pair(limits.send_buffer);
    let (session_id, mut sessions) = (member.session_id, hub.sessions.watch());
    let seat = Seat { member, role, origin, outbox: outbox.clone() };
    let Some(client_id) = enter(&mut socket, &hub, workbook, seat).await else {
        return;
    };
    let first_ping = tokio::time::Instant::now() + limits.ping;
    let (rate, mut ping) = (Rate::new(limits.pushes_per_second), tokio::time::interval_at(first_ping, limits.ping));
    let mut conn =
        Connection { hub, limits, workbook, client_id, role, outbox, rate, heard: Instant::now(), session_id };
    let flow = loop {
        let flow = tokio::select! {
            frame = inbox.recv() => conn.outbound(&mut socket, frame).await,
            message = socket.recv() => conn.inbound(message),
            _ = ping.tick() => conn.ping(&mut socket).await,
            _ = sessions.recv() => session::check(&conn.hub, conn.session_id).await,
        };
        if !matches!(flow, Flow::Continue) {
            break flow;
        }
    };
    if let Flow::Close(code, reason) = flow {
        close(&mut socket, code, &reason).await;
    }
    conn.hub.send(Command::Leave { workbook, client_id });
}

impl Connection {
    async fn outbound(&mut self, socket: &mut WebSocket, frame: Option<Frame>) -> Flow {
        let bytes = match frame {
            Some(Frame::Data(bytes) | Frame::Init(bytes)) => bytes,
            Some(Frame::Close(code, reason)) => return Flow::Close(code, reason),
            None => return Flow::Stop,
        };
        match tokio::time::timeout(self.limits.silence, socket.send(Message::Binary(bytes.into()))).await {
            Ok(Ok(())) => Flow::Continue,
            _ => Flow::Stop,
        }
    }

    fn inbound(&mut self, message: Option<Result<Message, axum::Error>>) -> Flow {
        self.heard = Instant::now();
        match message {
            Some(Ok(Message::Binary(bytes))) => self.frame(&bytes),
            Some(Ok(Message::Text(_))) => Flow::Close(codes::UNSUPPORTED, "binary frames only".into()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => Flow::Continue,
            Some(Err(error)) if too_long(&error) => too_big(),
            Some(Ok(Message::Close(_)) | Err(_)) | None => Flow::Stop,
        }
    }

    fn frame(&mut self, bytes: &[u8]) -> Flow {
        match protocol::decode(bytes) {
            Ok(ClientFrame::Push { client_seq, payload }) => self.push(client_seq, payload),
            Ok(ClientFrame::Cursor(cursor)) => {
                self.hub.send(Command::Cursor { workbook: self.workbook, client_id: self.client_id, cursor });
                Flow::Continue
            }
            Ok(ClientFrame::Hello(_)) | Err(FrameError::BadCursor) => Flow::Continue,
            Err(FrameError::Malformed) => Flow::Close(codes::PROTOCOL, "malformed frame".into()),
        }
    }

    fn push(&mut self, client_seq: u32, payload: Vec<u8>) -> Flow {
        if payload.len() > self.limits.max_push {
            return too_big();
        }
        let refused = match (self.role, self.rate.allow(Instant::now())) {
            (Role::Viewer, _) => Some("permission_denied"),
            (_, false) => Some("rate_limited"),
            _ => None,
        };
        match refused {
            Some(reason) => _ = self.outbox.send(protocol::reject(client_seq, reason)),
            None => {
                self.hub.send(Command::Push { workbook: self.workbook, client_id: self.client_id, client_seq, payload })
            }
        }
        Flow::Continue
    }

    async fn ping(&mut self, socket: &mut WebSocket) -> Flow {
        if self.heard.elapsed() >= self.limits.silence {
            return Flow::Close(codes::GOING_AWAY, "no frame in 60 seconds".into());
        }
        if let Flow::Close(code, reason) = session::check(&self.hub, self.session_id).await {
            return Flow::Close(code, reason);
        }
        match socket.send(Message::Ping(Vec::new().into())).await {
            Ok(()) => Flow::Continue,
            Err(_) => Flow::Stop,
        }
    }
}

/// Joins the room, or closes the socket: 1013 when it holds too many sockets, 1011 otherwise.
async fn enter(socket: &mut WebSocket, hub: &Hub, workbook: Uuid, seat: Seat) -> Option<u32> {
    match hub.ask(|reply| Command::Join { workbook, seat, reply }).await {
        Ok(Joined { client_id }) => Some(client_id),
        Err(error) if error.code == tronc::Code::ResourceExhausted => {
            close(socket, crate::hub::TRY_AGAIN_LATER, &error.message).await;
            None
        }
        Err(_) => {
            close(socket, codes::INTERNAL, "the workbook could not be opened").await;
            None
        }
    }
}

fn too_big() -> Flow {
    Flow::Close(codes::TOO_BIG, "a batch is at most 1 MiB".into())
}
