mod common;

use api::auth::{Eviction, apply_snapshot};
use common::directory::{member, members, snapshot, trashed, workbook};

#[tokio::test]
async fn snapshots_replace_spaces_and_members_and_report_who_lost_access() {
    let Some(state) = common::state().await else { return };
    let pool = &state.pool;
    reset(pool).await;
    first_snapshot(pool).await;
    let (in_s1, in_s2) = (workbook(pool, "s1").await, workbook(pool, "s2").await);
    let stale = snapshot(10, vec![("s1", vec![])]);
    assert!(apply_snapshot(pool, &stale).await.expect("apply").is_empty());
    assert_eq!(members(pool).await.len(), 3, "a snapshot that is not newer changes nothing");
    let second = snapshot(11, vec![("s1", vec![member("a", "owner")])]);
    let mut evictions = apply_snapshot(pool, &second).await.expect("apply");
    evictions.sort();
    let mut want =
        vec![Eviction { workbook: in_s1, subject: Some("b".into()) }, Eviction { workbook: in_s2, subject: None }];
    want.sort();
    assert_eq!(evictions, want);
    let left: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM workbooks").fetch_all(pool).await.expect("ids");
    assert_eq!(left.len(), 2, "a space Registre dropped keeps its workbooks");
    assert!(trashed(pool, in_s2).await.is_some(), "a dropped space's workbooks go to the trash");
    assert!(trashed(pool, in_s1).await.is_none());
    assert_eq!(members(pool).await.len(), 1);
    let name: String =
        sqlx::query_scalar("SELECT name FROM spaces WHERE id = 's1'").fetch_one(pool).await.expect("name");
    assert_eq!(name, "s1 name");
    assert_eq!(through_the_syncer(pool).await, [Eviction { workbook: in_s1, subject: Some("a".into()) }]);
}

async fn through_the_syncer(pool: &sqlx::PgPool) -> Vec<Eviction> {
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let record = std::sync::Arc::clone(&heard);
    let evict: api::auth::Evict = std::sync::Arc::new(move |e| record.lock().expect("lock").extend(e));
    let applier = api::auth::SpacesApply::new(pool.clone(), evict);
    porte::directory::Apply::apply(&applier, &snapshot(12, vec![("s1", vec![])])).await.expect("apply");
    heard.lock().expect("lock").clone()
}

async fn reset(pool: &sqlx::PgPool) {
    for table in ["directory_state", "workbooks", "spaces"] {
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}"))).execute(pool).await.expect("reset");
    }
}

async fn first_snapshot(pool: &sqlx::PgPool) {
    let first = snapshot(
        10,
        vec![("s1", vec![member("a", "owner"), member("b", "member")]), ("s2", vec![member("c", "boss")])],
    );
    assert_eq!(apply_snapshot(pool, &first).await.expect("apply"), Vec::<Eviction>::new());
    let rows = members(pool).await;
    assert_eq!(rows[0], ("s1".into(), "a".into(), "a@example.test".into(), "owner".into()));
    assert_eq!(rows[2].3, "member", "an unknown role is the member role");
}
