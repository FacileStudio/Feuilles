//! The `.xlsx` import shared by `POST /workbooks/import` and `api import-xlsx`.

use sqlx::PgPool;
use tronc::Error;

use super::service;
use super::store::{self, NewRow, Row};
use crate::engine::{self, Model};

/// The largest `.xlsx` an import takes.
pub const MAX_IMPORT_BYTES: usize = 20 << 20;

/// Locale, timezone and function-name language of an imported workbook.
pub const SETTINGS: (&str, &str, &str) = ("en", "UTC", "en");

/// The title an `.xlsx` file name gives: its stem, or `None` when it is not an `.xlsx`.
pub fn stem(name: &str) -> Option<&str> {
    name.strip_suffix(".xlsx").or_else(|| name.strip_suffix(".XLSX"))
}

/// The stored title for a file stem: trimmed, or "Imported workbook" when it does not fit.
pub fn title(stem: &str) -> String {
    service::title(stem).unwrap_or_else(|_| "Imported workbook".to_owned())
}

/// Reads an `.xlsx` with the import settings: 413 over `MAX_IMPORT_BYTES`, 400 when unreadable.
pub fn read(bytes: &[u8], title: &str) -> Result<Model, Error> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(Error::too_large("the file is larger than 20 MiB"));
    }
    let (locale, timezone, language) = SETTINGS;
    engine::import_xlsx(bytes, title, locale, timezone, language)
}

/// Stores an imported model as a new workbook at seq 0.
pub async fn store(
    pool: &PgPool,
    space_id: Option<&str>,
    owner_id: Option<i64>,
    title: &str,
    model: &Model,
) -> Result<Row, Error> {
    let new = NewRow { space_id, owner_id, title, settings: SETTINGS, snapshot: &model.to_bytes() };
    store::insert(pool, new).await
}
