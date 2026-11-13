use sqlx::PgPool;
use time::OffsetDateTime;
use tronc::Error;
use uuid::Uuid;

use super::types::Workbook;
use crate::engine::{ENGINE_VERSION, SheetInfo};

/// A `workbooks` row without its snapshot.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Row {
    /// The id.
    pub id: Uuid,
    /// The space, or `None` for a personal workbook.
    pub space_id: Option<String>,
    /// The owner of a personal workbook, or the creator of a space one.
    pub owner_id: Option<i64>,
    /// The title.
    pub title: String,
    /// The locale.
    pub locale: String,
    /// The timezone.
    pub timezone: String,
    /// The function-name language.
    pub language: String,
    /// The engine version of the stored bytes.
    pub engine_version: String,
    /// Creation time.
    pub created_at: OffsetDateTime,
    /// Last change.
    pub updated_at: OffsetDateTime,
    /// Trash time.
    pub trashed_at: Option<OffsetDateTime>,
}

const COLUMNS: &str =
    "id, space_id, owner_id, title, locale, timezone, language, engine_version, created_at, updated_at, trashed_at";

impl Row {
    /// The API shape, with `sheets`.
    pub fn into_workbook(self, sheets: Vec<SheetInfo>) -> Workbook {
        Workbook {
            id: self.id,
            space_id: self.space_id,
            title: self.title,
            locale: self.locale,
            timezone: self.timezone,
            language: self.language,
            sheets,
            engine_version: self.engine_version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            trashed_at: self.trashed_at,
        }
    }
}

fn failed(what: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::internal(what, error)
}

/// The row, or `None`.
pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<Row>, Error> {
    sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM workbooks WHERE id = $1")))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(failed("read a workbook"))
}

/// The workbooks of one scope: a space, or the personal workbooks of `owner`.
pub async fn list(pool: &PgPool, space: Option<&str>, owner: i64, trashed: bool) -> Result<Vec<Row>, Error> {
    let trash = if trashed { "IS NOT NULL" } else { "IS NULL" };
    let sql = format!(
        "SELECT {COLUMNS} FROM workbooks
         WHERE (space_id = $1 OR ($1 IS NULL AND space_id IS NULL AND owner_id = $2)) AND trashed_at {trash}
         ORDER BY updated_at DESC"
    );
    sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(sql))
        .bind(space)
        .bind(owner)
        .fetch_all(pool)
        .await
        .map_err(failed("list workbooks"))
}

/// What a new workbook row holds.
pub struct NewRow<'a> {
    /// The space, or `None`.
    pub space_id: Option<&'a str>,
    /// The creator, or `None` for a space workbook made by an operator command.
    pub owner_id: Option<i64>,
    /// The trimmed title.
    pub title: &'a str,
    /// Locale, timezone and language.
    pub settings: (&'a str, &'a str, &'a str),
    /// The first snapshot.
    pub snapshot: &'a [u8],
}

/// Inserts a workbook at seq 0.
pub async fn insert(pool: &PgPool, new: NewRow<'_>) -> Result<Row, Error> {
    let sql = format!(
        "INSERT INTO workbooks (space_id, owner_id, title, locale, timezone, language, engine_version, snapshot)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {COLUMNS}"
    );
    let (locale, timezone, language) = new.settings;
    sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(sql))
        .bind(new.space_id)
        .bind(new.owner_id)
        .bind(new.title)
        .bind(locale)
        .bind(timezone)
        .bind(language)
        .bind(ENGINE_VERSION)
        .bind(new.snapshot)
        .fetch_one(pool)
        .await
        .map_err(failed("create a workbook"))
}

/// What an update binds after the id.
#[derive(Debug, Clone, Copy)]
pub enum Bind<'a> {
    /// Nothing: the `SET` clause is complete.
    Nothing,
    /// `$2`, a title.
    Title(&'a str),
    /// `$2`, a space or `None` for personal, and `$3`, an owner.
    Scope(Option<&'a str>, i64),
}

/// Runs one `UPDATE workbooks SET <set> WHERE id = $1` and returns the row.
pub async fn update(pool: &PgPool, id: Uuid, set: &'static str, bind: Bind<'_>) -> Result<Row, Error> {
    let sql = format!("UPDATE workbooks SET {set} WHERE id = $1 RETURNING {COLUMNS}");
    let query = sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(sql)).bind(id);
    let query = match bind {
        Bind::Nothing => query,
        Bind::Title(title) => query.bind(title),
        Bind::Scope(space, owner) => query.bind(space).bind(owner),
    };
    query.fetch_optional(pool).await.map_err(failed("update a workbook"))?.ok_or_else(|| Error::not_found("not found"))
}

/// Deletes a trashed workbook, or answers 412 when it is not in the trash.
pub async fn delete_trashed(pool: &PgPool, id: Uuid) -> Result<(), Error> {
    let deleted = sqlx::query("DELETE FROM workbooks WHERE id = $1 AND trashed_at IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await
        .map_err(failed("delete a workbook"))?;
    match deleted.rows_affected() {
        0 => Err(Error::failed("only a workbook in the trash can be deleted for good")),
        _ => Ok(()),
    }
}

/// Deletes every workbook trashed before `before` and returns their ids.
pub async fn purge(pool: &PgPool, before: OffsetDateTime) -> Result<Vec<Uuid>, Error> {
    sqlx::query_scalar("DELETE FROM workbooks WHERE trashed_at < $1 RETURNING id")
        .bind(before)
        .fetch_all(pool)
        .await
        .map_err(failed("purge the trash"))
}
