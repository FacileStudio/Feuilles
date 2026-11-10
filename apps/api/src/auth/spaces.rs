use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use porte::directory::{Apply, DirectoryError, Snapshot, Space};
use sqlx::{PgConnection, PgPool};

use super::retire::{newer, refuse_empty, retire_spaces};
use uuid::Uuid;

/// Someone who lost access to a workbook: `subject`, or everyone when `None`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Eviction {
    /// The workbook.
    pub workbook: Uuid,
    /// The person, or `None` when the workbook itself is gone.
    pub subject: Option<String>,
}

/// What the app does with the people who lost access: close their live connections.
pub type Evict = Arc<dyn Fn(Vec<Eviction>) + Send + Sync>;

/// Applies Registre's directory snapshot to `spaces` and `space_members`.
pub struct SpacesApply {
    pool: PgPool,
    evict: Evict,
}

impl SpacesApply {
    /// The applier over `pool`, reporting evictions to `evict`.
    pub fn new(pool: PgPool, evict: Evict) -> SpacesApply {
        SpacesApply { pool, evict }
    }
}

#[async_trait::async_trait]
impl Apply for SpacesApply {
    async fn apply(&self, snapshot: &Snapshot) -> Result<(), DirectoryError> {
        let evictions =
            apply_snapshot(&self.pool, snapshot).await.map_err(|e| DirectoryError::Failed(e.to_string()))?;
        if !evictions.is_empty() {
            (self.evict)(evictions);
        }
        Ok(())
    }
}

type Pairs = BTreeSet<(String, String)>;

/// Applies `snapshot` in one transaction and returns who lost access to which workbook.
///
/// A snapshot whose version is not newer than the stored one changes nothing. A space the
/// snapshot no longer lists is never removed: it is marked deleted, its members are dropped, and
/// its workbooks go to the trash, where the 30-day purge is the only thing that deletes them. A
/// space that comes back is undeleted; its workbooks stay in the trash for a manager to restore.
/// An empty snapshot while live spaces exist is refused whole, logged at error level, and the
/// current state is kept until a non-empty snapshot arrives.
pub async fn apply_snapshot(pool: &PgPool, snapshot: &Snapshot) -> Result<Vec<Eviction>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !newer(&mut tx, snapshot.version).await? || refuse_empty(&mut tx, snapshot).await? {
        return Ok(Vec::new());
    }
    let before = pairs(&mut tx).await?;
    let kept: Vec<String> = snapshot.spaces.iter().map(|space| space.id.clone()).collect();
    let mut evictions = retire_spaces(&mut tx, &kept).await?;
    for space in &snapshot.spaces {
        replace_space(&mut tx, space).await?;
    }
    let lost: Pairs = before.difference(&pairs(&mut tx).await?).filter(|(s, _)| kept.contains(s)).cloned().collect();
    evictions.extend(lost_workbooks(&mut tx, &lost).await?);
    sqlx::query(
        "INSERT INTO directory_state (id, version) VALUES (1, $1)
         ON CONFLICT (id) DO UPDATE SET version = GREATEST(directory_state.version, EXCLUDED.version)",
    )
    .bind(snapshot.version)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(evictions)
}

async fn pairs(tx: &mut PgConnection) -> Result<Pairs, sqlx::Error> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT space_id, subject FROM space_members").fetch_all(tx).await?;
    Ok(rows.into_iter().collect())
}

async fn lost_workbooks(tx: &mut PgConnection, lost: &Pairs) -> Result<Vec<Eviction>, sqlx::Error> {
    let mut evictions = Vec::new();
    for (space, subject) in lost {
        let ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM workbooks WHERE space_id = $1").bind(space).fetch_all(&mut *tx).await?;
        evictions.extend(ids.into_iter().map(|workbook| Eviction { workbook, subject: Some(subject.clone()) }));
    }
    Ok(evictions)
}

async fn replace_space(tx: &mut PgConnection, space: &Space) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO spaces (id, name, updated_at) VALUES ($1, $2, now())
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, deleted_at = NULL, updated_at = now()",
    )
    .bind(&space.id)
    .bind(&space.name)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM space_members WHERE space_id = $1").bind(&space.id).execute(&mut *tx).await?;
    let members: BTreeMap<&str, _> =
        space.members.iter().filter(|m| !m.subject.is_empty()).map(|m| (m.subject.as_str(), m)).collect();
    let column = |pick: fn(&porte::directory::Member) -> String| members.values().map(|m| pick(m)).collect::<Vec<_>>();
    sqlx::query(
        "INSERT INTO space_members (space_id, subject, email, name, picture, role)
         SELECT $1, * FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[], $6::text[])",
    )
    .bind(&space.id)
    .bind(column(|m| m.subject.clone()))
    .bind(column(|m| m.email.trim().to_lowercase()))
    .bind(column(|m| m.name.trim().to_owned()))
    .bind(column(|m| m.picture.clone()))
    .bind(column(|m| local_role(&m.role).to_owned()))
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

/// Registre's rank as a local role. Anything unknown is the ordinary member role, never a rank
/// Registre did not grant.
pub fn local_role(role: &str) -> &'static str {
    match role {
        porte::directory::ROLE_OWNER => "owner",
        porte::directory::ROLE_ADMIN => "admin",
        _ => "member",
    }
}
