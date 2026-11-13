//! The durable form of a workbook: a snapshot at `snapshot_seq` plus every accepted batch after
//! it in `workbook_updates`. Memory is a cache of this, never the other way round.

use std::collections::HashMap;

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
    /// The highest accepted `client_seq` of every browser session that pushed here.
    pub acks: HashMap<Origin, u32>,
}

/// A browser session's random id, stable across its reconnects.
pub type Origin = [u8; 16];

/// One accepted batch, as stored.
pub struct Batch<'a> {
    /// Its place in the order.
    pub seq: u64,
    /// The person, or `None` when unknown.
    pub author: Option<i64>,
    /// The browser session and its counter, or `None` for a server write.
    pub origin: Option<(Origin, u32)>,
    /// The bytes.
    pub payload: &'a [u8],
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
    let acks = load_acks(pool, id).await?;
    Ok(Loaded { model, language, seq: unsigned(seq), snapshot_seq: unsigned(snapshot_seq), acks })
}

async fn load_acks(pool: &PgPool, id: Uuid) -> Result<HashMap<Origin, u32>, Error> {
    let rows: Vec<(Vec<u8>, i64)> =
        sqlx::query_as("SELECT origin, client_seq FROM workbook_acks WHERE workbook_id = $1")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(failed("read the acknowledgements"))?;
    let ack = |(origin, seq): (Vec<u8>, i64)| Some((Origin::try_from(origin).ok()?, u32::try_from(seq).ok()?));
    Ok(rows.into_iter().filter_map(ack).collect())
}

/// Stores one accepted batch, the workbook's `seq` and `sheet_count` (the `sheets` the model holds
/// after the batch) and the origin's acknowledgement, in one transaction.
pub async fn append(pool: &PgPool, id: Uuid, batch: &Batch<'_>, sheets: usize) -> Result<(), Error> {
    let mut tx = pool.begin().await.map_err(failed("store a batch"))?;
    sqlx::query(
        "INSERT INTO workbook_updates (workbook_id, seq, author_id, payload, origin, client_seq)
         VALUES ($1, $2, (SELECT id FROM porte_users WHERE id = $3), $4, $5, $6)",
    )
    .bind(id)
    .bind(signed(batch.seq))
    .bind(batch.author)
    .bind(batch.payload)
    .bind(batch.origin.map(|(origin, _)| origin.to_vec()))
    .bind(batch.origin.map(|(_, client_seq)| i64::from(client_seq)))
    .execute(&mut *tx)
    .await
    .map_err(failed("store a batch"))?;
    let updated = sqlx::query(
        "UPDATE workbooks SET seq = $2, sheet_count = $3, updated_at = now() WHERE id = $1 AND seq = $2 - 1",
    )
    .bind(id)
    .bind(signed(batch.seq))
    .bind(i32::try_from(sheets).unwrap_or(i32::MAX))
    .execute(&mut *tx)
    .await
    .map_err(failed("store a batch"))?;
    if updated.rows_affected() != 1 {
        return Err(Error::internal("store a batch", format!("workbook {id} is not at seq {}", batch.seq - 1)));
    }
    if let Some((origin, client_seq)) = batch.origin {
        store_ack(&mut tx, id, &origin, client_seq).await?;
    }
    tx.commit().await.map_err(failed("store a batch"))
}

async fn store_ack(tx: &mut sqlx::PgConnection, id: Uuid, origin: &Origin, client_seq: u32) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO workbook_acks (workbook_id, origin, client_seq) VALUES ($1, $2, $3)
         ON CONFLICT (workbook_id, origin) DO UPDATE SET client_seq = EXCLUDED.client_seq, updated_at = now()",
    )
    .bind(id)
    .bind(origin.as_slice())
    .bind(i64::from(client_seq))
    .execute(tx)
    .await
    .map(|_| ())
    .map_err(failed("store a batch"))
}

/// Replaces the snapshot with `bytes` at `seq` and drops the batches it now includes. Does
/// nothing when the stored snapshot is already newer or the log is behind `seq`: a stale writer
/// (a second process, an `engine-export` against a live database) must never move it back.
pub async fn snapshot(pool: &PgPool, id: Uuid, bytes: &[u8], seq: u64) -> Result<(), Error> {
    let mut tx = pool.begin().await.map_err(failed("write a snapshot"))?;
    let written = sqlx::query(
        "UPDATE workbooks SET snapshot = $2, snapshot_seq = $3 WHERE id = $1 AND snapshot_seq <= $3 AND seq >= $3",
    )
    .bind(id)
    .bind(bytes)
    .bind(signed(seq))
    .execute(&mut *tx)
    .await
    .map_err(failed("write a snapshot"))?;
    if written.rows_affected() == 0 {
        return tx.commit().await.map_err(failed("write a snapshot"));
    }
    sqlx::query("DELETE FROM workbook_updates WHERE workbook_id = $1 AND seq <= $2")
        .bind(id)
        .bind(signed(seq))
        .execute(&mut *tx)
        .await
        .map_err(failed("write a snapshot"))?;
    tx.commit().await.map_err(failed("write a snapshot"))
}
