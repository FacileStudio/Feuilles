use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use porte::Identity;
use tronc::Error;
use tronc::json::Json;

use super::imports::{self, Format, MAX_IMPORT_BYTES};
use super::model;
use super::service::{Access, parse_id, require, space_scope};
use super::types::SheetQuery;
use crate::engine;
use crate::state::AppState;

const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// Imports that may parse at once. Each can hold a few hundred MiB while IronCalc reads it.
static IMPORTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

struct Upload {
    name: String,
    bytes: Vec<u8>,
    space_id: Option<String>,
    format: Format,
}

fn multipart_error(error: axum::extract::multipart::MultipartError) -> Error {
    match error.status() {
        StatusCode::PAYLOAD_TOO_LARGE => Error::too_large("the file is larger than 20 MiB"),
        _ => Error::invalid(format!("invalid multipart body: {}", error.body_text())),
    }
}

async fn field_bytes(field: Field<'_>) -> Result<Vec<u8>, Error> {
    let bytes = field.bytes().await.map_err(multipart_error)?;
    match bytes.len() > MAX_IMPORT_BYTES {
        true => Err(Error::too_large("the file is larger than 20 MiB")),
        false => Ok(bytes.to_vec()),
    }
}

async fn read_upload(multipart: &mut Multipart) -> Result<Upload, Error> {
    let mut upload = Upload { name: String::new(), bytes: Vec::new(), space_id: None, format: Format::Xlsx };
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") => {
                upload.name = field.file_name().unwrap_or_default().to_owned();
                upload.bytes = field_bytes(field).await?;
            }
            Some("space_id") => upload.space_id = Some(field.text().await.map_err(multipart_error)?),
            other => return Err(Error::invalid(format!("unknown field {:?}", other.unwrap_or_default()))),
        }
    }
    match (imports::stem(&upload.name), upload.bytes.is_empty()) {
        (Some((stem, format)), false) => Ok(Upload { name: stem.to_owned(), format, ..upload }),
        _ => Err(Error::invalid("send one .xlsx or .csv file in the field \"file\"")),
    }
}

/// `POST /workbooks/import`: multipart `file` (`.xlsx` or `.csv`, at most 20 MiB) and `space_id`.
pub async fn import(
    State(state): State<AppState>,
    identity: Identity,
    mut multipart: Multipart,
) -> Result<Response, Error> {
    let upload = read_upload(&mut multipart).await?;
    let space = space_scope(&state, &identity, upload.space_id.as_deref()).await?;
    let _turn = IMPORTS.acquire().await.map_err(|e| Error::internal("wait for an import slot", e))?;
    let title = imports::title(&upload.name);
    let (bytes, name, format) = (upload.bytes, title.clone(), upload.format);
    let model = tokio::task::spawn_blocking(move || imports::read(&bytes, &name, format))
        .await
        .map_err(|e| Error::internal("import a file", e))??;
    let row = imports::store(&state.pool, space.as_deref(), Some(identity.user_id), &title, &model).await?;
    Ok((StatusCode::CREATED, Json(row.into_workbook(engine::sheets(&model)))).into_response())
}

/// `GET /workbooks/{id}/export.xlsx`.
pub async fn export_xlsx(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
) -> Result<Response, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Viewer).await?;
    let bytes = model::export(&state, row.id, engine::export_xlsx).await?;
    Ok(attachment(XLSX, &format!("{}.xlsx", row.title), bytes))
}

/// `GET /workbooks/{id}/export.csv?sheet=0`.
pub async fn export_csv(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
    Query(query): Query<SheetQuery>,
) -> Result<Response, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Viewer).await?;
    let sheet = query.sheet;
    let text = model::export(&state, row.id, move |current| engine::export_csv(current, sheet)).await?;
    Ok(attachment("text/csv; charset=utf-8", &format!("{}.csv", row.title), text.into_bytes()))
}

/// A download answer with an ASCII `filename` and the exact UTF-8 `filename*`.
pub fn attachment(content_type: &'static str, name: &str, body: Vec<u8>) -> Response {
    let ascii: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || " .-_".contains(c) { c } else { '_' }).collect();
    let encoded: String = name
        .bytes()
        .map(|b| match b.is_ascii_alphanumeric() || b".-_".contains(&b) {
            true => char::from(b).to_string(),
            false => format!("%{b:02X}"),
        })
        .collect();
    let disposition = format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}");
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}
