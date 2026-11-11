//! The guards of a directory snapshot: the version check, the empty-snapshot refusal, and the
//! soft delete of spaces Registre no longer lists.

use porte::directory::Snapshot;
use sqlx::PgConnection;
use uuid::Uuid;

use super::spaces::Eviction;

pub(super) async fn newer(tx: &mut PgConnection, version: i64) -> Result<bool, sqlx::Error> {
    let stored: Option<i64> =
        sqlx::query_scalar("SELECT version FROM directory_state WHERE id = 1 FOR UPDATE").fetch_optional(tx).await?;
    Ok(version <= 0 || stored.is_none_or(|stored| stored < version))
}

pub(super) async fn refuse_empty(tx: &mut PgConnection, snapshot: &Snapshot) -> Result<bool, sqlx::Error> {
    if !snapshot.spaces.is_empty() {
        return Ok(false);
    }
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM spaces WHERE deleted_at IS NULL").fetch_one(tx).await?;
    if live > 0 {
        tracing::error!(
            version = snapshot.version,
            live,
            "directory: refused an empty snapshot while spaces exist; keeping the current state"
        );
    }
    Ok(live > 0)
}

pub(super) async fn retire_spaces(tx: &mut PgConnection, kept: &[String]) -> Result<Vec<Eviction>, sqlx::Error> {
    let gone: Vec<String> = sqlx::query_scalar(
        "UPDATE spaces SET deleted_at = now(), updated_at = now() WHERE deleted_at IS NULL AND id <> ALL($1)
         RETURNING id",
    )
    .bind(kept)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM space_members WHERE space_id = ANY($1)").bind(&gone).execute(&mut *tx).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE workbooks SET trashed_at = COALESCE(trashed_at, now()) WHERE space_id = ANY($1) RETURNING id",
    )
    .bind(&gone)
    .fetch_all(&mut *tx)
    .await?;
    Ok(ids.into_iter().map(|workbook| Eviction { workbook: Some(workbook), subject: None }).collect())
}
