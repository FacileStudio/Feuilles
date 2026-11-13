mod common;

use std::time::Duration;

use api::hub::Limits;
use common::sheet::{batch, init};
use common::web::{call, create};
use common::ws::{Event, connect};
use common::{ISSUER, person, serve, space, state, state_with};
use http::{Method, StatusCode};
use porte::directory::PersonDeleted;

#[tokio::test(flavor = "multi_thread")]
async fn a_logout_closes_the_sessions_sockets_at_once() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (me, other) = (person(&state).await, person(&state).await);
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let other_id = create(&router, &other, None).await["id"].as_str().expect("id").to_owned();
    let (mut mine, mut theirs) = (connect(addr, &id, &me.token).await, connect(addr, &other_id, &other.token).await);
    init(&mut mine).await;
    let (_, _, mut model) = init(&mut theirs).await;
    let (status, _) = call(&router, Method::POST, "/api/auth/logout", Some(&me.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mine.closed().await, Some(1008), "the default ping is 25 s: only the logout signal closes this fast");
    theirs.push(1, &batch(&mut model, 1, "still here")).await;
    assert!(matches!(theirs.next_data().await, Event::Update { seq: 1, .. }), "another session stays open");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_expired_session_closes_at_the_next_ping() {
    let limits = Limits { ping: Duration::from_millis(200), ..Limits::default() };
    let Some(state) = state_with(limits).await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let mut ws = connect(addr, &id, &me.token).await;
    init(&mut ws).await;
    sqlx::query("UPDATE porte_sessions SET expires_at = now() - interval '1 second' WHERE user_id = $1")
        .bind(me.user_id)
        .execute(&state.pool)
        .await
        .expect("expire");
    assert_eq!(ws.closed().await, Some(1008));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_person_cannot_break_the_room_and_loses_every_socket() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (owner, member) = (person(&state).await, person(&state).await);
    let shared = space(&state, &[(&owner, "owner"), (&member, "member")]).await;
    let id = create(&router, &owner, Some(&shared)).await["id"].as_str().expect("id").to_owned();
    let (mut kept, mut gone) = (connect(addr, &id, &owner.token).await, connect(addr, &id, &member.token).await);
    init(&mut kept).await;
    let (_, _, mut model) = init(&mut gone).await;
    sqlx::query("DELETE FROM porte_users WHERE id = $1").bind(member.user_id).execute(&state.pool).await.expect("drop");
    gone.push(1, &batch(&mut model, 1, "late")).await;
    assert!(
        matches!(kept.next_data().await, Event::Update { seq: 1, .. }),
        "a vanished author must not close with 1011"
    );
    let eraser = api::auth::AppData::new(state.pool.clone(), api::modules::live::evictor(state.hub.clone()));
    eprintln!("DBG before erase");
    eraser.on_person_deleted(ISSUER, &member.subject).await.expect("erase");
    eprintln!("DBG erased");
    for _ in 0..3 {
        let event = tokio::time::timeout(Duration::from_secs(4), gone.next()).await;
        eprintln!("DBG gone {event:?}");
    }
    assert_eq!(gone.closed().await, Some(1008), "a space workbook socket closes when its person is deleted");
}
