use porte::directory::{Member, Snapshot, Space};

pub fn member(subject: &str, role: &str) -> Member {
    Member {
        subject: subject.into(),
        email: format!(" {}@Example.TEST ", subject.to_uppercase()),
        role: role.into(),
        ..Member::default()
    }
}

pub fn snapshot(version: i64, spaces: Vec<(&str, Vec<Member>)>) -> Snapshot {
    let spaces = spaces.into_iter().map(|(id, members)| Space {
        id: id.into(),
        name: format!("{id} name"),
        members,
        ..Space::default()
    });
    Snapshot { version, spaces: spaces.collect(), ..Snapshot::default() }
}

pub async fn workbook(pool: &sqlx::PgPool, space: &str) -> uuid::Uuid {
    sqlx::query_scalar(
        "INSERT INTO workbooks (space_id, title, engine_version, snapshot) VALUES ($1, 't', '0.9.1', '') RETURNING id",
    )
    .bind(space)
    .fetch_one(pool)
    .await
    .expect("workbook")
}

pub async fn members(pool: &sqlx::PgPool) -> Vec<(String, String, String, String)> {
    sqlx::query_as("SELECT space_id, subject, email, role FROM space_members ORDER BY space_id, subject")
        .fetch_all(pool)
        .await
        .expect("members")
}

pub async fn version(pool: &sqlx::PgPool) -> Option<i64> {
    sqlx::query_scalar("SELECT version FROM directory_state WHERE id = 1").fetch_optional(pool).await.expect("version")
}

pub async fn deleted(pool: &sqlx::PgPool, space: &str) -> bool {
    let at: Option<time::OffsetDateTime> = sqlx::query_scalar("SELECT deleted_at FROM spaces WHERE id = $1")
        .bind(space)
        .fetch_one(pool)
        .await
        .expect("space");
    at.is_some()
}

pub async fn trashed(pool: &sqlx::PgPool, workbook: uuid::Uuid) -> Option<time::OffsetDateTime> {
    sqlx::query_scalar("SELECT trashed_at FROM workbooks WHERE id = $1")
        .bind(workbook)
        .fetch_one(pool)
        .await
        .expect("row")
}
