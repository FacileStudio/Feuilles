mod common;

use std::time::Duration;

use api::auth::{SpacesApply, apply_snapshot};
use api::hub::Limits;
use api::state::AppState;
use common::directory::{deleted, member, members, snapshot, trashed, version, workbook};
use common::sheet::init;
use common::web::{call, create};
use common::ws::connect;
use common::{person, serve, state_in};
use http::{Method, StatusCode};
use time::OffsetDateTime;

async fn isolated(rule: &str) -> Option<AppState> {
    let state = state_in(&format!("feuilles_{rule}"), Limits::default()).await?;
    tronc::testdb::truncate(&state.pool, &api::MIGRATOR, &[]).await.expect("truncate");
    Some(state)
}

#[tokio::test]
async fn a_space_missing_from_a_snapshot_is_marked_deleted_and_its_workbooks_trashed() {
    let Some(state) = isolated("gone").await else { return };
    let pool = &state.pool;
    let one = vec![("s1", vec![member("a", "owner")]), ("s2", vec![member("c", "member")])];
    apply_snapshot(pool, &snapshot(1, one)).await.expect("first");
    let (kept, dropped, older) = (workbook(pool, "s1").await, workbook(pool, "s2").await, workbook(pool, "s2").await);
    let ten_days_ago = OffsetDateTime::now_utc() - Duration::from_secs(10 * 24 * 3600);
    set_trashed(pool, older, ten_days_ago).await;
    let evicted = evicted(pool, snapshot(2, vec![("s1", vec![member("a", "owner")])])).await;
    let mut want = vec![(dropped, None), (older, None)];
    want.sort();
    assert_eq!(evicted, want, "every socket of a dropped space's workbooks closes");
    assert!(deleted(pool, "s2").await, "the space row stays, marked deleted");
    assert!(!deleted(pool, "s1").await);
    assert!(members(pool).await.iter().all(|m| m.0 == "s1"), "a deleted space has no members");
    assert!(trashed(pool, dropped).await.is_some());
    let kept_date = trashed(pool, older).await.map(OffsetDateTime::unix_timestamp);
    assert_eq!(kept_date, Some(ten_days_ago.unix_timestamp()), "an earlier trash date is kept");
    assert!(trashed(pool, kept).await.is_none());
    only_the_purge_deletes(&state, dropped).await;
}

async fn set_trashed(pool: &sqlx::PgPool, workbook: uuid::Uuid, at: OffsetDateTime) {
    sqlx::query("UPDATE workbooks SET trashed_at = $2 WHERE id = $1")
        .bind(workbook)
        .bind(at)
        .execute(pool)
        .await
        .expect("trash");
}

async fn evicted(pool: &sqlx::PgPool, next: porte::directory::Snapshot) -> Vec<(uuid::Uuid, Option<String>)> {
    let evictions = apply_snapshot(pool, &next).await.expect("apply");
    let mut pairs: Vec<_> = evictions.into_iter().map(|e| (e.workbook.unwrap_or_default(), e.subject)).collect();
    pairs.sort();
    pairs
}

async fn only_the_purge_deletes(state: &AppState, workbook: uuid::Uuid) {
    assert_eq!(api::purge::purge_once(state).await.expect("purge"), 0, "nothing is deleted before 30 days");
    let long_ago = OffsetDateTime::now_utc() - api::purge::RETENTION - Duration::from_secs(60);
    set_trashed(&state.pool, workbook, long_ago).await;
    assert_eq!(api::purge::purge_once(state).await.expect("purge"), 1, "the 30-day purge is the only delete");
}

#[tokio::test]
async fn an_empty_snapshot_is_refused_while_spaces_exist() {
    let Some(state) = isolated("empty").await else { return };
    let pool = &state.pool;
    assert!(apply_snapshot(pool, &snapshot(1, vec![])).await.expect("empty on nothing").is_empty());
    assert_eq!(version(pool).await, Some(1), "an empty snapshot applies when no space exists");
    apply_snapshot(pool, &snapshot(2, vec![("s1", vec![member("a", "owner")])])).await.expect("one space");
    let in_s1 = workbook(pool, "s1").await;
    assert!(apply_snapshot(pool, &snapshot(3, vec![])).await.expect("empty").is_empty());
    assert_eq!(version(pool).await, Some(2), "a refused snapshot does not move the version");
    assert!(!deleted(pool, "s1").await, "a refused snapshot deletes no space");
    assert_eq!(members(pool).await.len(), 1);
    assert!(trashed(pool, in_s1).await.is_none());
    let next = snapshot(4, vec![("s1", vec![member("a", "owner"), member("b", "member")])]);
    apply_snapshot(pool, &next).await.expect("non-empty");
    assert_eq!((version(pool).await, members(pool).await.len()), (Some(4), 2), "the next real snapshot applies");
}

#[tokio::test]
async fn a_space_that_comes_back_is_undeleted_and_its_workbooks_stay_in_the_trash() {
    let Some(state) = isolated("back").await else { return };
    let (pool, router) = (&state.pool, api::router::build_router(state.clone(), Default::default()).expect("router"));
    let owner = person(&state).await;
    let space = vec![("s1", vec![member(&owner.subject, "owner")])];
    apply_snapshot(pool, &snapshot(1, space.clone())).await.expect("first");
    let id = create(&router, &owner, Some("s1")).await["id"].as_str().expect("id").to_owned();
    apply_snapshot(pool, &snapshot(2, vec![("s9", vec![])])).await.expect("dropped");
    let path = format!("/api/workbooks/{id}");
    assert_eq!(call(&router, Method::GET, &path, Some(&owner.token), None).await.0, StatusCode::NOT_FOUND);
    apply_snapshot(pool, &snapshot(3, space)).await.expect("back");
    assert!(!deleted(pool, "s1").await, "the space is undeleted");
    let (status, body) =
        call(&router, Method::GET, "/api/workbooks?space_id=s1&trashed=true", Some(&owner.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["workbooks"][0]["id"], id.as_str(), "the workbook waits in the trash");
    let restore = format!("{path}/restore");
    assert_eq!(call(&router, Method::POST, &restore, Some(&owner.token), None).await.0, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn members_of_a_deleted_space_lose_access_at_once() {
    let Some(state) = isolated("evict").await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (owner, editor) = (person(&state).await, person(&state).await);
    let space = vec![("s1", vec![member(&owner.subject, "owner"), member(&editor.subject, "member")])];
    apply_snapshot(&state.pool, &snapshot(1, space)).await.expect("first");
    let id = create(&router, &owner, Some("s1")).await["id"].as_str().expect("id").to_owned();
    let (mut a, mut b) = (connect(addr, &id, &owner.token).await, connect(addr, &id, &editor.token).await);
    init(&mut a).await;
    init(&mut b).await;
    let applier = SpacesApply::new(state.pool.clone(), api::modules::live::evictor(state.hub.clone()));
    porte::directory::Apply::apply(&applier, &snapshot(2, vec![("s9", vec![])])).await.expect("apply");
    assert_eq!((a.closed().await, b.closed().await), (Some(1008), Some(1008)));
    for who in [&owner, &editor] {
        let path = format!("/api/workbooks/{id}");
        assert_eq!(call(&router, Method::GET, &path, Some(&who.token), None).await.0, StatusCode::NOT_FOUND);
        assert_eq!(connect(addr, &id, &who.token).await.closed().await, Some(1008));
    }
}
