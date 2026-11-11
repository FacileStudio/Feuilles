mod common;

use common::sheet::{batch, init};
use common::web::{call, create};
use common::ws::{Event, connect, connect_as, open};
use common::{person, serve, space, state};
use http::{Method, StatusCode};

#[tokio::test(flavor = "multi_thread")]
async fn a_resent_batch_is_not_applied_twice_and_init_carries_the_ack() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let origin = *uuid::Uuid::new_v4().as_bytes();
    let mut ws = connect_as(addr, &id, &me.token, origin).await;
    let Event::Init { acked: 0, snapshot, .. } = ws.next_data().await else { panic!("INIT without ack 0") };
    let mut model = common::sheet::model(&snapshot);
    model.insert_rows(0, 1, 1).expect("insert");
    let inserted = model.flush_send_queue();
    ws.push(5, &inserted).await;
    assert!(matches!(ws.next_data().await, Event::Update { seq: 1, client_seq: 5, .. }));
    ws.push(5, &inserted).await;
    ws.push(6, &batch(&mut model, 1, "after")).await;
    assert!(matches!(ws.next_data().await, Event::Update { seq: 2, client_seq: 6, .. }), "the duplicate was applied");
    drop(ws);
    let mut again = connect_as(addr, &id, &me.token, origin).await;
    let Event::Init { seq, acked, .. } = again.next_data().await else { panic!("no INIT") };
    assert_eq!((seq, acked), (2, 6));
    again.push(6, &inserted).await;
    let mut other = connect(addr, &id, &me.token).await;
    let Event::Init { seq, acked, .. } = other.next_data().await else { panic!("no INIT") };
    assert_eq!((seq, acked), (2, 0), "a resend under an acked client_seq is dropped, another origin starts at 0");
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_upgrades_close_with_1008_and_a_reason() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (owner, outsider) = (person(&state).await, person(&state).await);
    let shared = space(&state, &[(&owner, "owner")]).await;
    let id = create(&router, &owner, Some(&shared)).await["id"].as_str().expect("id").to_owned();
    let cases = [
        (id.clone(), Some(outsider.token.clone()), "not_found"),
        (uuid::Uuid::new_v4().to_string(), Some(owner.token.clone()), "not_found"),
        ("not-a-uuid".to_owned(), Some(owner.token.clone()), "not_found"),
        (id.clone(), None, "unauthenticated"),
    ];
    for (workbook, token, reason) in cases {
        let bearer = token.map(|t| format!("Bearer {t}"));
        let headers: Vec<(&str, &str)> = bearer.iter().map(|b| ("authorization", b.as_str())).collect();
        let mut ws = open(addr, &workbook, &headers).await.expect("the upgrade completes");
        let frame = futures_util::StreamExt::next(&mut ws.0).await.expect("frame").expect("close");
        let tokio_tungstenite::tungstenite::Message::Close(Some(close)) = frame else { panic!("expected a close") };
        assert_eq!((u16::from(close.code), close.reason.as_str()), (1008, reason), "{workbook}");
    }
    let mut ws = connect(addr, &id, &owner.token).await;
    assert_eq!(init(&mut ws).await.1, 2);
    let path = format!("/api/workbooks/{id}/ws");
    assert_eq!(call(&router, Method::GET, &path, Some(&outsider.token), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&router, Method::GET, &path, None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&router, Method::GET, &path, Some(&owner.token), None).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread")]
async fn losing_access_closes_with_1008_and_a_failed_store_with_1011() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (owner, member) = (person(&state).await, person(&state).await);
    let shared = space(&state, &[(&owner, "owner"), (&member, "member")]).await;
    let id = create(&router, &owner, Some(&shared)).await["id"].as_str().expect("id").to_owned();
    let (mut kept, mut gone) = (connect(addr, &id, &owner.token).await, connect(addr, &id, &member.token).await);
    let (_, _, mut model) = init(&mut kept).await;
    init(&mut gone).await;
    let eviction =
        api::auth::Eviction { workbook: Some(id.parse().expect("uuid")), subject: Some(member.subject.clone()) };
    api::modules::live::evictor(state.hub.clone())(vec![eviction]);
    assert_eq!(gone.closed().await, Some(1008));
    sqlx::query("DELETE FROM workbooks WHERE id = $1::uuid").bind(&id).execute(&state.pool).await.expect("delete");
    kept.push(1, &batch(&mut model, 1, "lost")).await;
    assert_eq!(kept.closed().await, Some(1011));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cookie_upgrade_must_come_from_the_apps_own_origin() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let cookie = format!("session={}", me.token);
    let host = addr.to_string();
    let foreign = open(addr, &id, &[("cookie", &cookie), ("origin", "https://evil.example")]).await;
    assert!(foreign.is_err(), "a cross-origin upgrade with the cookie was accepted");
    let own = format!("http://{host}");
    let mut ws = open(addr, &id, &[("cookie", &cookie), ("origin", &own)]).await.expect("same origin");
    ws.send([&[0x03][..], &[1; 16]].concat()).await;
    assert_eq!(init(&mut ws).await.1, 2);
}
