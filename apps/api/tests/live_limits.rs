mod common;

use std::time::Duration;

use api::hub::Limits;
use common::sheet::{batch, init};
use common::web::{call, create};
use common::ws::{Event, Ws, connect};
use common::{Person, person, serve, state_with};
use http::Method;

async fn opened(limits: Limits) -> Option<(api::state::AppState, std::net::SocketAddr, Person, String)> {
    let state = state_with(limits).await?;
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    Some((state, addr, me, id))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_batch_reloads_the_room_so_it_matches_the_database() {
    let Some((state, addr, me, id)) = opened(Limits::default()).await else { return };
    let mut ws = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut ws).await;
    let initial = model.to_bytes();
    model.new_sheet().expect("sheet");
    let _never_sent = model.flush_send_queue();
    let area = ironcalc::base::expressions::types::Area { sheet: 1, row: 1, column: 1, width: 1, height: 1 };
    model.update_range_style(&area, "font.b", "true").expect("bold");
    let refused = model.flush_send_queue();
    let mut probe = common::sheet::model(&initial);
    assert!(probe.apply_external_diffs(&refused).is_err());
    assert_ne!(probe.to_bytes(), initial, "this batch must change a model even as it is refused");
    ws.push(1, &refused).await;
    assert_eq!(ws.next_data().await, Event::Reject { client_seq: 1, reason: "invalid_argument".into() });
    ws.push(2, b"not a batch").await;
    assert_eq!(ws.next_data().await, Event::Reject { client_seq: 2, reason: "invalid_argument".into() });
    let mut late = connect(addr, &id, &me.token).await;
    let Event::Init { seq, snapshot, .. } = late.next_data().await else { panic!("no INIT") };
    let stored = api::modules::workbooks::history::load(&state.pool, id.parse().expect("uuid")).await.expect("load");
    assert_eq!((seq, snapshot), (0, stored.model.to_bytes()), "the room kept a model the database does not have");
    let mut fresh = common::sheet::model(&stored.model.to_bytes());
    ws.push(3, &batch(&mut fresh, 1, "ok")).await;
    assert!(matches!(ws.next_data().await, Event::Update { seq: 1, client_seq: 3, .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_viewer_is_refused_and_too_many_pushes_are_rate_limited() {
    let Some((state, addr, me, id)) = opened(Limits::default()).await else { return };
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let mut editor = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut editor).await;
    let payload = batch(&mut model, 1, "1");
    for client_seq in 0..70 {
        editor.push(client_seq, &payload).await;
    }
    let mut limited = 0;
    for _ in 0..70 {
        match editor.next_data().await {
            Event::Reject { reason, .. } if reason == "rate_limited" => limited += 1,
            Event::Update { .. } => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(limited >= 10, "{limited} pushes were rate limited");
    call(&router, Method::DELETE, &format!("/api/workbooks/{id}"), Some(&me.token), None).await;
    assert_eq!(editor.closed().await, Some(1012));
    let mut viewer = connect(addr, &id, &me.token).await;
    let (_, role, _) = init(&mut viewer).await;
    assert_eq!(role, 1, "a workbook in the trash opens read-only");
    viewer.push(9, &payload).await;
    assert_eq!(viewer.next_data().await, Event::Reject { client_seq: 9, reason: "permission_denied".into() });
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_text_malformed_and_silent_sockets_are_closed() {
    let limits = Limits { silence: Duration::from_millis(600), ping: Duration::from_millis(100), ..Limits::default() };
    let Some((_, addr, me, id)) = opened(limits).await else { return };
    let mut big = connect(addr, &id, &me.token).await;
    init(&mut big).await;
    big.push(1, &vec![0u8; (1 << 20) + 1]).await;
    assert_eq!(big.closed().await, Some(1009));
    let mut text = connect(addr, &id, &me.token).await;
    futures_util::SinkExt::send(&mut text.0, tokio_tungstenite::tungstenite::Message::text("hi")).await.expect("send");
    assert_eq!(text.closed().await, Some(1003));
    let mut malformed = connect(addr, &id, &me.token).await;
    malformed.send(vec![0x7F]).await;
    assert_eq!(malformed.closed().await, Some(1002));
    let mut silent: Ws = connect(addr, &id, &me.token).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(silent.closed().await, Some(1001));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_socket_that_cannot_keep_up_is_closed_with_1013() {
    let limits = Limits { send_buffer: 512 << 10, ..Limits::default() };
    let Some((_, addr, me, id)) = opened(limits).await else { return };
    let mut writer = connect(addr, &id, &me.token).await;
    let mut slow = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut writer).await;
    let (mut sink, mut stream) = futures_util::StreamExt::split(writer.0);
    tokio::spawn(async move { while futures_util::StreamExt::next(&mut stream).await.is_some() {} });
    let text = "x".repeat(25_000);
    for round in 0..200u32 {
        let writes: Vec<_> = (1..=8)
            .map(|row| api::engine::CellWrite {
                sheet: 0,
                row,
                column: i32::try_from(round).unwrap_or(0) + 1,
                input: text.clone(),
            })
            .collect();
        api::engine::set_inputs(&mut model, &writes).expect("inputs");
        let frame = [&[0x01][..], &round.to_be_bytes(), &model.flush_send_queue()].concat();
        futures_util::SinkExt::send(&mut sink, tokio_tungstenite::tungstenite::Message::Binary(frame.into()))
            .await
            .expect("push");
        tokio::time::sleep(Duration::from_millis(18)).await;
    }
    assert_eq!(slow.closed().await, Some(1013));
}

#[tokio::test(flavor = "multi_thread")]
async fn any_client_frame_keeps_a_socket_alive() {
    let limits = Limits { silence: Duration::from_millis(600), ping: Duration::from_millis(100), ..Limits::default() };
    let Some((_, addr, me, id)) = opened(limits).await else { return };
    let mut ws = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut ws).await;
    let cursor = br#"{"sheet":0,"row":1,"column":1,"range":[1,1,1,1]}"#;
    let (mut sink, stream) = futures_util::StreamExt::split(ws.0);
    for _ in 0..8 {
        let frame = [&[0x02][..], cursor].concat();
        futures_util::SinkExt::send(&mut sink, tokio_tungstenite::tungstenite::Message::Binary(frame.into()))
            .await
            .expect("cursor");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let push = [&[0x01][..], &1u32.to_be_bytes(), &batch(&mut model, 1, "alive")].concat();
    futures_util::SinkExt::send(&mut sink, tokio_tungstenite::tungstenite::Message::Binary(push.into()))
        .await
        .expect("push");
    let mut ws = Ws(sink.reunite(stream).expect("reunite"));
    assert!(matches!(ws.next_data().await, Event::Update { seq: 1, .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_batch_past_the_sheet_cap_is_refused_and_the_room_reloads() {
    let Some((state, addr, me, id)) = opened(Limits { max_sheets: 2, ..Limits::default() }).await else { return };
    let mut ws = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut ws).await;
    model.new_sheet().expect("sheet");
    model.new_sheet().expect("sheet");
    ws.push(1, &model.flush_send_queue()).await;
    assert_eq!(ws.next_data().await, Event::Reject { client_seq: 1, reason: "too_large".into() });
    let mut late = connect(addr, &id, &me.token).await;
    let Event::Init { seq, snapshot, .. } = late.next_data().await else { panic!("no INIT") };
    assert_eq!((seq, api::engine::sheet_count(&common::sheet::model(&snapshot))), (0, 1));
    let stored = api::modules::workbooks::history::load(&state.pool, id.parse().expect("uuid")).await.expect("load");
    assert_eq!(stored.seq, 0, "the refused batch was stored");
}

#[tokio::test(flavor = "multi_thread")]
async fn one_person_cannot_open_more_sockets_than_the_cap() {
    let Some((_, addr, me, id)) = opened(Limits { max_user_sockets: 2, ..Limits::default() }).await else { return };
    let (mut first, mut second) = (connect(addr, &id, &me.token).await, connect(addr, &id, &me.token).await);
    init(&mut first).await;
    init(&mut second).await;
    let mut third = connect(addr, &id, &me.token).await;
    assert_eq!(third.closed().await, Some(1013));
}
