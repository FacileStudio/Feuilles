mod common;

use api::auth::erase_person;
use common::{ISSUER, person, space};
use time::{Duration, OffsetDateTime};

async fn workbook(pool: &sqlx::PgPool, space: Option<&str>, owner: i64) -> uuid::Uuid {
    sqlx::query_scalar(
        "INSERT INTO workbooks (space_id, owner_id, title, engine_version, snapshot) VALUES ($1, $2, 't', '0.9.1', '') RETURNING id",
    )
    .bind(space)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("workbook")
}

async fn exists(pool: &sqlx::PgPool, sql: &str, id: impl ToString) -> bool {
    sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(sql.to_owned()))
        .bind(id.to_string())
        .fetch_one(pool)
        .await
        .expect("exists")
}

#[tokio::test]
async fn erasing_a_person_takes_their_personal_workbooks_and_leaves_space_ones() {
    let Some(state) = common::state().await else { return };
    let (gone, other) = (person(&state).await, person(&state).await);
    let shared = space(&state, &[(&gone, "owner"), (&other, "member")]).await;
    let personal = workbook(&state.pool, None, gone.user_id).await;
    let in_space = workbook(&state.pool, Some(&shared), gone.user_id).await;
    let erased = erase_person(&state.pool, ISSUER, &gone.subject).await.expect("erase");
    assert_eq!(erased, [personal]);
    let pool = &state.pool;
    assert!(!exists(pool, "SELECT EXISTS (SELECT 1 FROM workbooks WHERE id = $1::uuid)", personal).await);
    assert!(
        exists(pool, "SELECT EXISTS (SELECT 1 FROM workbooks WHERE id = $1::uuid AND owner_id IS NULL)", in_space)
            .await
    );
    assert!(!exists(pool, "SELECT EXISTS (SELECT 1 FROM space_members WHERE subject = $1)", &gone.subject).await);
    assert!(exists(pool, "SELECT EXISTS (SELECT 1 FROM space_members WHERE subject = $1)", &other.subject).await);
    assert!(!exists(pool, "SELECT EXISTS (SELECT 1 FROM porte_users WHERE id = $1::bigint)", gone.user_id).await);
}

#[tokio::test]
async fn the_purge_deletes_only_workbooks_trashed_over_30_days_ago() {
    let Some(state) = common::state().await else { return };
    let me = person(&state).await;
    let (old, recent, live) = (
        workbook(&state.pool, None, me.user_id).await,
        workbook(&state.pool, None, me.user_id).await,
        workbook(&state.pool, None, me.user_id).await,
    );
    for (id, age) in [(old, Duration::days(31)), (recent, Duration::days(29))] {
        sqlx::query("UPDATE workbooks SET trashed_at = $2 WHERE id = $1")
            .bind(id)
            .bind(OffsetDateTime::now_utc() - age)
            .execute(&state.pool)
            .await
            .expect("trash");
    }
    assert!(api::purge::purge_once(&state).await.expect("purge") >= 1);
    let left: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM workbooks WHERE owner_id = $1 ORDER BY created_at")
        .bind(me.user_id)
        .fetch_all(&state.pool)
        .await
        .expect("left");
    assert!(!left.contains(&old) && left.contains(&recent) && left.contains(&live));
}

#[test]
fn the_first_migration_is_porte_schema_verbatim() {
    assert_eq!(include_str!("../migrations/0001_porte.sql"), porte::pg::SCHEMA);
}
