use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket};

use super::socket::{Flow, codes};
use crate::hub::Origin;
use crate::hub::protocol::{self, ClientFrame, FrameError};

const CLOSE_WAIT: Duration = Duration::from_secs(2);

/// Closes a socket the caller may not open with 1008 and `reason`, so the client stops
/// retrying: `unauthenticated` or `not_found`.
pub async fn refuse(mut socket: WebSocket, reason: &'static str) {
    close(&mut socket, codes::POLICY, reason).await;
}

pub(super) async fn hello(socket: &mut WebSocket, silence: Duration) -> Result<Origin, Flow> {
    let first = async {
        loop {
            match socket.recv().await {
                Some(Ok(Message::Binary(bytes))) => return protocol::decode(&bytes),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                _ => return Err(FrameError::Malformed),
            }
        }
    };
    match tokio::time::timeout(silence, first).await {
        Ok(Ok(ClientFrame::Hello(origin))) => Ok(origin),
        Ok(_) => Err(Flow::Close(codes::PROTOCOL, "the first frame must be HELLO".into())),
        Err(_) => Err(Flow::Close(codes::GOING_AWAY, "no HELLO in time".into())),
    }
}

/// Sends a close frame, then waits a little for the client's, so it reads the code.
pub(super) async fn close(socket: &mut WebSocket, code: u16, reason: &str) {
    let frame = CloseFrame { code, reason: reason.into() };
    if socket.send(Message::Close(Some(frame))).await.is_err() {
        return;
    }
    let answered = async {
        while let Some(Ok(message)) = socket.recv().await
            && !matches!(message, Message::Close(_))
        {}
    };
    let _ = tokio::time::timeout(CLOSE_WAIT, answered).await;
}

/// Whether a receive error is tungstenite refusing a message over the size limit.
pub(super) fn too_long(error: &axum::Error) -> bool {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if matches!(current.downcast_ref::<tungstenite::Error>(), Some(tungstenite::Error::Capacity(_))) {
            return true;
        }
        source = current.source();
    }
    false
}
