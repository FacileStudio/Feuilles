use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub use crate::engine::CellWrite;
use crate::engine::{Cell, SheetInfo};

/// One workbook as the API answers it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct Workbook {
    /// Its id.
    pub id: Uuid,
    /// The space it belongs to, or null for a personal workbook.
    pub space_id: Option<String>,
    /// Its title, 1 to 200 characters.
    pub title: String,
    /// The number and date locale, such as `en`.
    pub locale: String,
    /// The IANA timezone, such as `UTC`.
    pub timezone: String,
    /// The language of function names, such as `en`.
    pub language: String,
    /// Its sheets in tab order. Empty in list answers: read one workbook for its sheets.
    pub sheets: Vec<SheetInfo>,
    /// The engine version its bytes are written with. The browser engine must match it.
    pub engine_version: String,
    /// When it was created.
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub created_at: OffsetDateTime,
    /// When it last changed.
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub updated_at: OffsetDateTime,
    /// When it went to the trash, or null.
    #[serde(with = "time::serde::rfc3339::option")]
    #[schemars(with = "Option<String>")]
    pub trashed_at: Option<OffsetDateTime>,
}

/// `{"workbooks":[Workbook]}`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WorkbookList {
    /// Newest change first.
    pub workbooks: Vec<Workbook>,
}

/// `GET /workbooks` query.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    /// The space, or absent for personal workbooks.
    pub space_id: Option<String>,
    /// `true` lists the trash instead.
    #[serde(default)]
    pub trashed: bool,
}

/// `POST /workbooks` body.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkbook {
    /// 1 to 200 characters after trimming.
    pub title: String,
    /// The space, or absent for a personal workbook.
    pub space_id: Option<String>,
    /// Default `en`.
    pub locale: Option<String>,
    /// Default `UTC`.
    pub timezone: Option<String>,
    /// Default `en`.
    pub language: Option<String>,
}

/// `PATCH /workbooks/{id}` body.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameWorkbook {
    /// 1 to 200 characters after trimming.
    pub title: String,
}

/// `GET /workbooks/{id}/export.csv` query.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct SheetQuery {
    /// The sheet index, default 0.
    #[serde(default)]
    pub sheet: u32,
}

/// `GET /workbooks/{id}/cells` query.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct CellsQuery {
    /// The sheet index, default 0.
    #[serde(default)]
    pub sheet: u32,
    /// Such as `A1:D20`, at most 10 000 cells.
    pub range: String,
}

/// `GET /workbooks/{id}/cells` answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CellsAnswer {
    /// The sheet index.
    pub sheet: u32,
    /// The normalised range, such as `A1:D20`.
    pub range: String,
    /// The cells, row by row.
    pub rows: Vec<Vec<Cell>>,
}

/// `PUT /workbooks/{id}/cells` body.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteCells {
    /// 1 to 5000 writes, applied in order as one batch.
    pub writes: Vec<CellWrite>,
}

/// `PUT /workbooks/{id}/cells` answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WriteAnswer {
    /// The sequence number of the batch the writes became.
    pub seq: u64,
}
