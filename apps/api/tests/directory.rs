mod common;

use api::auth::{Eviction, apply_snapshot};
use porte::directory::{Member, Snapshot, Space};

fn member(subject: &str, role: &str) -> Member {
    Member {
        subject: subject.into(),
        email: format!(" {}@Example.TEST ", subject.to_uppercase()),
        role: role.into(),
        ..Member::default()
    }
}

fn snapshot(version: i64, spaces: Vec<(&str, Vec<Member>)>) -> Snapshot {
    let spaces = spaces.into_iter().map(|(id, members)| Space {
        id: id.into(),
        name: format!("{id} name"),
        members,
        ..Space::default()
    });
    Snapshot { version, spaces: spaces.collect(), ..Snapshot::default() }
}

async fn workbook(pool: &sqlx::PgPool, space: &str) -> uuid::Uuid {
    sqlx::query_scalar(
        "INSERT INTO workbooks (space_id, title, engine_version, snapshot) VALUES ($1, 't', '0.9.1', '') RETURNING id",
    )
    .bind(space)
    .fetch_one(pool)
    .await
    .expect("workbook")
}

async fn members(pool: &sqlx::PgPool) -> Vec<(String, String, String, String)> {
    sqlx::query_as("SELECT space_id, subject, email, role FROM space_members ORDER BY space_id, subject")
        .fetch_all(pool)
        .await
        .expect("members")
}

#[tokio::test]
async fn snapshots_replace_spaces_and_members_and_report_who_lost_access() {
    let Some(state) = common::state().await else { return };
    let pool = &state.pool;
    for table in ["directory_state", "workbooks", "spaces"] {
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}"))).execute(pool).await.expect("reset");
    }
    let first = snapshot(
        10,
        vec![("s1", vec![member("a", "owner"), member("b", "member")]), ("s2", vec![member("c", "boss")])],
    );
    assert_eq!(apply_snapshot(pool, &first).await.expect("apply"), Vec::<Eviction>::new());
    let rows = members(pool).await;
    assert_eq!(rows[0], ("s1".into(), "a".into(), "a@example.test".into(), "owner".into()));
    assert_eq!(rows[2].3, "member", "an unknown role is the member role");
    let (in_s1, in_s2) = (workbook(pool, "s1").await, workbook(pool, "s2").await);
    let stale = snapshot(10, vec![]);
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
    assert_eq!(left, [in_s1], "a space Registre deleted takes its workbooks with it");
    assert_eq!(members(pool).await.len(), 1);
    let name: String =
        sqlx::query_scalar("SELECT name FROM spaces WHERE id = 's1'").fetch_one(pool).await.expect("name");
    assert_eq!(name, "s1 name");
}
