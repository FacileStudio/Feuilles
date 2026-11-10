use std::net::SocketAddr;
use std::time::Duration;

pub use super::frames::Event;
use super::frames::event;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub struct Ws(pub WebSocketStream<MaybeTlsStream<TcpStream>>);

pub async fn open(addr: SocketAddr, workbook: &str, headers: &[(&str, &str)]) -> Result<Ws, Error> {
    let mut request = format!("ws://{addr}/api/workbooks/{workbook}/ws").into_client_request()?;
    for (name, value) in headers {
        let name = http::HeaderName::from_bytes(name.as_bytes()).expect("name");
        request.headers_mut().insert(name, value.parse().expect("header"));
    }
    let (stream, _) = tokio_tungstenite::connect_async(request).await?;
    Ok(Ws(stream))
}

pub async fn connect(addr: SocketAddr, workbook: &str, token: &str) -> Ws {
    connect_as(addr, workbook, token, *uuid::Uuid::new_v4().as_bytes()).await
}

pub async fn connect_as(addr: SocketAddr, workbook: &str, token: &str, origin: [u8; 16]) -> Ws {
    let mut ws = open(addr, workbook, &[("authorization", &format!("Bearer {token}"))]).await.expect("connect");
    ws.send([&[0x03][..], &origin].concat()).await;
    ws
}

impl Ws {
    pub async fn next(&mut self) -> Event {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.0.next()).await.expect("a frame in time");
            match message {
                Some(Ok(Message::Binary(bytes))) => return event(&bytes),
                Some(Ok(Message::Close(frame))) => return Event::Closed(frame.map(|f| u16::from(f.code))),
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => return Event::Closed(None),
            }
        }
    }

    pub async fn next_data(&mut self) -> Event {
        loop {
            match self.next().await {
                Event::Presence(_) => continue,
                event => return event,
            }
        }
    }

    pub async fn send(&mut self, frame: Vec<u8>) {
        self.0.send(Message::Binary(frame.into())).await.expect("send");
    }

    pub async fn push(&mut self, client_seq: u32, payload: &[u8]) {
        self.send([&[0x01][..], &client_seq.to_be_bytes(), payload].concat()).await;
    }

    pub async fn closed(&mut self) -> Option<u16> {
        loop {
            if let Event::Closed(code) = self.next().await {
                return code;
            }
        }
    }
}
