//! The `.xlsx` and `.csv` import shared by `POST /workbooks/import` and `api import-xlsx`.

use sqlx::PgPool;
use tronc::Error;

use super::service;
use super::store::{self, NewRow, Row};
use crate::engine::{self, Model};

/// The largest file an import takes.
pub const MAX_IMPORT_BYTES: usize = 20 << 20;

/// Locale, timezone and function-name language of an imported workbook.
pub const SETTINGS: (&str, &str, &str) = ("en", "UTC", "en");

/// The file formats an import reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// An Excel workbook, every sheet kept.
    Xlsx,
    /// One sheet of comma, semicolon or tab separated values.
    Csv,
}

/// The stem and format of a file name ending in `.xlsx` or `.csv`, in any case, or `None`.
pub fn stem(name: &str) -> Option<(&str, Format)> {
    let (stem, extension) = name.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "xlsx" => Some((stem, Format::Xlsx)),
        "csv" => Some((stem, Format::Csv)),
        _ => None,
    }
}

/// The stored title for a file stem: trimmed, or "Imported workbook" when it does not fit.
pub fn title(stem: &str) -> String {
    service::title(stem).unwrap_or_else(|_| "Imported workbook".to_owned())
}

/// Reads a file with the import settings: 413 over `MAX_IMPORT_BYTES`, 400 when unreadable.
/// A CSV's one sheet is named after `title`.
pub fn read(bytes: &[u8], title: &str, format: Format) -> Result<Model, Error> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(Error::too_large("the file is larger than 20 MiB"));
    }
    let (locale, timezone, language) = SETTINGS;
    match format {
        Format::Xlsx => engine::import_xlsx(bytes, title, locale, timezone, language),
        Format::Csv => engine::import_csv(bytes, title, locale, timezone, language),
    }
}

/// Stores an imported model as a new workbook at seq 0.
pub async fn store(
    pool: &PgPool,
    space_id: Option<&str>,
    owner_id: Option<i64>,
    title: &str,
    model: &Model,
) -> Result<Row, Error> {
    let new = NewRow { space_id, owner_id, title, settings: SETTINGS, model };
    store::insert(pool, new).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_xlsx_and_csv_names_have_a_stem() {
        assert_eq!(stem("Q4 plan.XLSX"), Some(("Q4 plan", Format::Xlsx)));
        assert_eq!(stem("a.b.Csv"), Some(("a.b", Format::Csv)));
        assert_eq!(stem(".csv"), Some(("", Format::Csv)));
        assert_eq!(stem("notes.txt"), None);
        assert_eq!(stem("csv"), None);
    }
}
