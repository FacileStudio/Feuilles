//! The durable form of a workbook: a snapshot at `snapshot_seq` plus every accepted batch after
//! it in `workbook_updates`. Memory is a cache of this, never the other way round.

use sqlx::PgPool;
use tronc::Error;
use uuid::Uuid;

use crate::engine::{self, Model};

/// A model rebuilt from the database.
pub struct Loaded {
    /// The model at `seq`.
    pub model: Model,
    /// The language it was loaded with.
    pub language: &'static str,
    /// The last accepted batch.
    pub seq: u64,
    /// The batch the stored snapshot includes.
    pub snapshot_seq: u64,
}

fn failed(what: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::internal(what, error)
}

fn unsigned(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn signed(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Reads the snapshot and replays the batches after it, in order. 404 for an unknown id.
pub async fn load(pool: &PgPool, id: Uuid) -> Result<Loaded, Error> {
    let row: Option<(Vec<u8>, i64, i64, String)> =
        sqlx::query_as("SELECT snapshot, snapshot_seq, seq, language FROM workbooks WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(failed("read a snapshot"))?;
    let (snapshot, snapshot_seq, seq, language) = row.ok_or_else(|| Error::not_found("not found"))?;
    let language = engine::language(&language)?;
    let mut model = engine::load(&snapshot, language)?;
    let updates: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT seq, payload FROM workbook_updates WHERE workbook_id = $1 AND seq > $2 ORDER BY seq")
            .bind(id)
            .bind(snapshot_seq)
            .fetch_all(pool)
            .await
            .map_err(failed("read the update log"))?;
    let mut applied = snapshot_seq;
    for (at, payload) in updates {
        model.apply_external_diffs(&payload).map_err(|e| Error::internal("replay the update log", e))?;
        applied = at;
    }
    if applied != seq {
        return Err(Error::internal("replay the update log", format!("log ends at {applied}, workbook at {seq}")));
    }
    Ok(Loaded { model, language, seq: unsigned(seq), snapshot_seq: unsigned(snapshot_seq) })
}

/// Stores one accepted batch as `seq`, in one transaction with the workbook's `seq`.
pub async fn append(pool: &PgPool, id: Uuid, seq: u64, author: Option<i64>, payload: &[u8]) -> Result<(), Error> {
    let mut tx = pool.begin().await.map_err(failed("store a batch"))?;
    sqlx::query("INSERT INTO workbook_updates (workbook_id, seq, author_id, payload) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(signed(seq))
        .bind(author)
        .bind(payload)
        .execute(&mut *tx)
        .await
        .map_err(failed("store a batch"))?;
    let updated = sqlx::query("UPDATE workbooks SET seq = $2, updated_at = now() WHERE id = $1 AND seq = $2 - 1")
        .bind(id)
        .bind(signed(seq))
        .execute(&mut *tx)
        .await
        .map_err(failed("store a batch"))?;
    if updated.rows_affected() != 1 {
        return Err(Error::internal("store a batch", format!("workbook {id} is not at seq {}", seq - 1)));
    }
    tx.commit().await.map_err(failed("store a batch"))
}

/// Replaces the snapshot with `bytes` at `seq` and drops the batches it now includes.
pub async fn snapshot(pool: &PgPool, id: Uuid, bytes: &[u8], seq: u64) -> Result<(), Error> {
    let mut tx = pool.begin().await.map_err(failed("write a snapshot"))?;
    sqlx::query("UPDATE workbooks SET snapshot = $2, snapshot_seq = $3 WHERE id = $1")
        .bind(id)
        .bind(bytes)
        .bind(signed(seq))
        .execute(&mut *tx)
        .await
        .map_err(failed("write a snapshot"))?;
    sqlx::query("DELETE FROM workbook_updates WHERE workbook_id = $1 AND seq <= $2")
        .bind(id)
        .bind(signed(seq))
        .execute(&mut *tx)
        .await
        .map_err(failed("write a snapshot"))?;
    tx.commit().await.map_err(failed("write a snapshot"))
}
