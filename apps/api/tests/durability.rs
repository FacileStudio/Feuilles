mod common;

use std::collections::BTreeSet;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::web::create;
use common::ws::{Event, Ws, connect_as};
use common::{person, state};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

fn spawn_api(port: u16) -> Child {
    let url = tronc::testdb::url().expect("TEST_DATABASE_URL");
    let joiner = if url.contains('?') { '&' } else { '?' };
    let schema = tronc::testdb::schema_name("feuilles");
    Command::new(env!("CARGO_BIN_EXE_api"))
        .env("DATABASE_URL", format!("{url}{joiner}options=-c%20search_path%3D{schema}"))
        .env("PORT", port.to_string())
        .env("CLIENT_DIR", "/nonexistent")
        .env("LOG_LEVEL", "warn")
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn the api")
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port()
}

async fn join(port: u16, id: &str, token: &str, origin: [u8; 16]) -> Ws {
    let addr = format!("127.0.0.1:{port}").parse().expect("addr");
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let probe = tokio::net::TcpStream::connect(addr).await;
        if probe.is_ok() {
            return connect_as(addr, id, token, origin).await;
        }
        assert!(Instant::now() < deadline, "the api did not come up");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn every_acknowledged_batch_survives_a_kill_during_a_burst() {
    let Some(state) = state().await else { return };
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let (port, origin) = (free_port(), *uuid::Uuid::new_v4().as_bytes());
    let mut child = spawn_api(port);
    let mut ws = join(port, &id, &me.token, origin).await;
    let Event::Init { snapshot, .. } = ws.next_data().await else { panic!("no INIT") };
    let mut model = common::sheet::model(&snapshot);
    let (mut sink, stream) = ws.0.split();
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reader = tokio::spawn(acknowledged(stream, std::sync::Arc::clone(&seen)));
    let mut killed = false;
    for client_seq in 1..=1000u32 {
        let row = i32::try_from(client_seq).expect("row");
        model.set_user_input(0, row, 1, &client_seq.to_string()).expect("input");
        let frame = [&[0x01][..], &client_seq.to_be_bytes(), &model.flush_send_queue()].concat();
        if sink.send(Message::Binary(frame.into())).await.is_err() {
            break;
        }
        if !killed && seen.load(std::sync::atomic::Ordering::SeqCst) >= 20 {
            child.kill().expect("kill -9");
            killed = true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(killed, "the burst ended before 20 batches were acknowledged");
    let acked = tokio::time::timeout(Duration::from_secs(10), reader).await.expect("reader ends").expect("reader");
    child.wait().expect("reap");
    restarted(port, &id, &me.token, origin, &acked).await;
}

type Stream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

async fn acknowledged(mut stream: Stream, seen: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> BTreeSet<u32> {
    let mut acked = BTreeSet::new();
    while let Some(Ok(message)) = stream.next().await {
        if let Message::Binary(frame) = message
            && frame[0] == 0x11
        {
            acked.insert(u32::from_be_bytes(frame[13..17].try_into().expect("client_seq")));
            seen.store(acked.len(), std::sync::atomic::Ordering::SeqCst);
        }
    }
    acked
}

async fn restarted(port: u16, id: &str, token: &str, origin: [u8; 16], acked: &BTreeSet<u32>) {
    assert!(acked.len() >= 10, "only {} batches were acknowledged before the kill", acked.len());
    let mut child = spawn_api(port);
    let mut ws = join(port, id, token, origin).await;
    let Event::Init { acked: last, snapshot, .. } = ws.next_data().await else { panic!("no INIT after restart") };
    let restored = common::sheet::model(&snapshot);
    for client_seq in acked {
        let row = i32::try_from(*client_seq).expect("row");
        assert_eq!(
            restored.get_cell_content(0, row, 1).expect("cell"),
            client_seq.to_string(),
            "batch {client_seq} lost"
        );
    }
    assert!(last >= *acked.last().expect("acked"));
    child.kill().expect("kill");
    child.wait().expect("reap");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stale_snapshot_never_moves_the_stored_one_back() {
    let Some(state) = state().await else { return };
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id: uuid::Uuid = create(&router, &me, None).await["id"].as_str().expect("id").parse().expect("uuid");
    let at =
        |seq: i64| sqlx::query("UPDATE workbooks SET seq = $2, snapshot_seq = $2 WHERE id = $1").bind(id).bind(seq);
    at(5).execute(&state.pool).await.expect("seq");
    api::modules::workbooks::history::snapshot(&state.pool, id, b"stale", 3).await.expect("snapshot");
    let (seq, bytes): (i64, Vec<u8>) = sqlx::query_as("SELECT snapshot_seq, snapshot FROM workbooks WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .expect("row");
    assert_eq!(seq, 5);
    assert_ne!(bytes, b"stale");
}
