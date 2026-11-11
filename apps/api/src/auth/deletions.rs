use porte::directory::{DirectoryError, PersonDeleted, StateStore};
use sqlx::PgPool;
use uuid::Uuid;

use super::spaces::{Evict, Eviction};

/// Erases what Feuilles holds about a person Registre deleted: their personal workbooks, their
/// space memberships and their account. Space workbooks stay, with no owner. Their sockets close
/// in every room.
pub struct AppData {
    pool: PgPool,
    evict: Evict,
}

impl AppData {
    /// The eraser over `pool`, closing the erased workbooks through `evict`.
    pub fn new(pool: PgPool, evict: Evict) -> AppData {
        AppData { pool, evict }
    }
}

/// Erases `(provider, subject)` in one transaction and returns the personal workbooks it
/// deleted.
pub async fn erase_person(pool: &PgPool, provider: &str, subject: &str) -> Result<Vec<Uuid>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let user: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM porte_identities WHERE provider = $1 AND subject = $2")
            .bind(provider)
            .bind(subject)
            .fetch_optional(&mut *tx)
            .await?;
    let mut erased = Vec::new();
    if let Some(user) = user {
        erased = sqlx::query_scalar("DELETE FROM workbooks WHERE space_id IS NULL AND owner_id = $1 RETURNING id")
            .bind(user)
            .fetch_all(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM porte_users WHERE id = $1").bind(user).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM space_members WHERE subject = $1").bind(subject).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(erased)
}

#[async_trait::async_trait]
impl PersonDeleted for AppData {
    async fn on_person_deleted(&self, provider: &str, subject: &str) -> Result<(), DirectoryError> {
        let erased =
            erase_person(&self.pool, provider, subject).await.map_err(|e| DirectoryError::Failed(e.to_string()))?;
        let mut evictions: Vec<Eviction> =
            erased.into_iter().map(|workbook| Eviction { workbook: Some(workbook), subject: None }).collect();
        evictions.push(Eviction { workbook: None, subject: Some(subject.to_owned()) });
        (self.evict)(evictions);
        Ok(())
    }
}

/// The deletion receiver's cursor, in the one-row `deletion_state` table.
pub struct DeletionState {
    pool: PgPool,
}

impl DeletionState {
    /// The state over `pool`.
    pub fn new(pool: PgPool) -> DeletionState {
        DeletionState { pool }
    }
}

#[async_trait::async_trait]
impl StateStore for DeletionState {
    async fn load(&self) -> Result<Vec<u8>, DirectoryError> {
        let state: Option<Vec<u8>> = sqlx::query_scalar("SELECT state FROM deletion_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DirectoryError::Failed(e.to_string()))?;
        Ok(state.unwrap_or_default())
    }

    async fn save(&self, state: &[u8]) -> Result<(), DirectoryError> {
        sqlx::query(
            "INSERT INTO deletion_state (id, state, updated_at) VALUES (1, $1, now())
             ON CONFLICT (id) DO UPDATE SET state = EXCLUDED.state, updated_at = now()",
        )
        .bind(state)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| DirectoryError::Failed(e.to_string()))
    }
}
