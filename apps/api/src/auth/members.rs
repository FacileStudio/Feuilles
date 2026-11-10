use porte::spaces::{Membership, Role, Seeder, SpaceError, Store};
use sqlx::PgPool;

/// The `spaces` and `space_members` tables as a [`porte::spaces::Store`]. Users are keyed by
/// their Registre subject, the key the directory snapshot carries.
#[derive(Debug, Clone)]
pub struct Members {
    pool: PgPool,
}

impl Members {
    /// The store over `pool`.
    pub fn new(pool: PgPool) -> Members {
        Members { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    space_id: String,
    subject: String,
    role: String,
}

impl From<Row> for Membership {
    fn from(row: Row) -> Membership {
        Membership { space_id: row.space_id, user_id: row.subject, role: Role(row.role) }
    }
}

fn failed(error: sqlx::Error) -> SpaceError {
    SpaceError::Store(error.to_string())
}

#[async_trait::async_trait]
impl Store for Members {
    async fn membership(&self, space_id: &str, user_id: &str) -> Result<Membership, SpaceError> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT space_id, subject, role FROM space_members WHERE space_id = $1 AND subject = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(failed)?;
        row.map(Membership::from).ok_or(SpaceError::NotMember)
    }

    async fn memberships(&self, user_id: &str) -> Result<Vec<Membership>, SpaceError> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT space_id, subject, role FROM space_members WHERE subject = $1 ORDER BY space_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(failed)?;
        Ok(rows.into_iter().map(Membership::from).collect())
    }

    async fn count_role(&self, space_id: &str, role: &Role) -> Result<usize, SpaceError> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM space_members WHERE space_id = $1 AND role = $2")
            .bind(space_id)
            .bind(role.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(failed)?;
        Ok(usize::try_from(count).unwrap_or(0))
    }
}

#[async_trait::async_trait]
impl Seeder for Members {
    async fn seed(&self, member: &Membership) -> Result<(), SpaceError> {
        let mut tx = self.pool.begin().await.map_err(failed)?;
        sqlx::query("INSERT INTO spaces (id, name) VALUES ($1, $1) ON CONFLICT (id) DO NOTHING")
            .bind(&member.space_id)
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        sqlx::query(
            "INSERT INTO space_members (space_id, subject, email, role) VALUES ($1, $2, '', $3)
             ON CONFLICT (space_id, subject) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(&member.space_id)
        .bind(&member.user_id)
        .bind(member.role.as_str())
        .execute(&mut *tx)
        .await
        .map_err(failed)?;
        tx.commit().await.map_err(failed)
    }
}
