//! Counts the sheets of the workbooks stored before `workbooks.sheet_count` existed. Every
//! batch and every new workbook writes the count itself, so this only fills old rows.

use std::time::Duration;

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tronc::Error;
use uuid::Uuid;

use crate::engine;
use crate::modules::workbooks::history;

/// Workbooks read per query.
pub const PAGE: i64 = 50;

/// The pause after each workbook, so the backfill never competes with the rooms.
pub const PAUSE: Duration = Duration::from_millis(50);

fn failed(what: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::internal(what, error)
}

/// Fills `sheet_count` where it is null, loading one workbook at a time straight from the
/// database, never through a room, until none is left or `shutdown`. Returns how many it filled.
/// A workbook that does not load is logged and skipped.
pub async fn fill(pool: &PgPool, shutdown: &CancellationToken, pause: Duration) -> Result<usize, Error> {
    let (mut after, mut filled) = (Uuid::nil(), 0);
    loop {
        let ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM workbooks WHERE sheet_count IS NULL AND id > $1 ORDER BY id LIMIT $2")
                .bind(after)
                .bind(PAGE)
                .fetch_all(pool)
                .await
                .map_err(failed("list uncounted workbooks"))?;
        let Some(last) = ids.last() else { return Ok(filled) };
        after = *last;
        for id in ids {
            if shutdown.is_cancelled() {
                return Ok(filled);
            }
            filled += count(pool, id).await?;
            tokio::time::sleep(pause).await;
        }
    }
}

async fn count(pool: &PgPool, id: Uuid) -> Result<usize, Error> {
    let loaded = match history::load(pool, id).await {
        Ok(loaded) => loaded,
        Err(error) => {
            tracing::warn!(workbook = %id, error = %error.message, "sheet count: workbook did not load");
            return Ok(0);
        }
    };
    let sheets = i32::try_from(engine::sheet_count(&loaded.model)).unwrap_or(i32::MAX);
    let seq = i64::try_from(loaded.seq).unwrap_or(i64::MAX);
    let written =
        sqlx::query("UPDATE workbooks SET sheet_count = $2 WHERE id = $1 AND sheet_count IS NULL AND seq = $3")
            .bind(id)
            .bind(sheets)
            .bind(seq)
            .execute(pool)
            .await
            .map_err(failed("store a sheet count"))?;
    Ok(usize::try_from(written.rows_affected()).unwrap_or(0))
}

/// Runs [`fill`] once at boot on a blocking thread: loading a workbook is CPU work that must not
/// hold an async worker the rooms' database calls need.
pub async fn run(pool: PgPool, shutdown: CancellationToken) {
    let handle = tokio::runtime::Handle::current();
    let task = tokio::task::spawn_blocking(move || handle.block_on(fill(&pool, &shutdown, PAUSE)));
    match task.await {
        Ok(Ok(0)) => {}
        Ok(Ok(count)) => tracing::info!(count, "sheet count: backfilled workbooks"),
        Ok(Err(error)) => tracing::warn!(error = %error.message, "sheet count: backfill failed"),
        Err(error) => tracing::warn!(%error, "sheet count: backfill stopped"),
    }
}
