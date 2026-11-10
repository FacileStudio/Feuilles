use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use porte::Identity;
use tronc::Error;
use tronc::json::Json;

use super::service::{self, Access, parse_id, require};
use super::store;
use super::types::{CreateWorkbook, ListQuery, RenameWorkbook, Workbook, WorkbookList};
use super::{actions, model};
use crate::modules::live;
use crate::state::AppState;

/// `GET /workbooks`.
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
    Query(query): Query<ListQuery>,
) -> Result<Json<WorkbookList>, Error> {
    let workbooks = actions::list(&state, &identity, query.space_id.as_deref(), query.trashed).await?;
    Ok(Json(WorkbookList { workbooks }))
}

/// `POST /workbooks`.
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    Json(body): Json<CreateWorkbook>,
) -> Result<Response, Error> {
    let workbook = actions::create(&state, &identity, &body).await?;
    Ok((StatusCode::CREATED, Json(workbook)).into_response())
}

/// `GET /workbooks/{id}`.
pub async fn get(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
) -> Result<Json<Workbook>, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Viewer).await?;
    let sheets = model::sheets(&state, row.id).await?;
    Ok(Json(row.into_workbook(sheets)))
}

/// `PATCH /workbooks/{id}`.
pub async fn rename(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
    Json(body): Json<RenameWorkbook>,
) -> Result<Json<Workbook>, Error> {
    let title = service::title(&body.title)?;
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Editor).await?;
    let row = store::update(&state.pool, row.id, "title = $2, updated_at = now()", Some(&title)).await?;
    let sheets = model::sheets(&state, row.id).await?;
    Ok(Json(row.into_workbook(sheets)))
}

/// `DELETE /workbooks/{id}`: to the trash.
pub async fn trash(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
) -> Result<StatusCode, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Manager).await?;
    store::update(&state.pool, row.id, "trashed_at = COALESCE(trashed_at, now())", None).await?;
    live::reopen(&state, row.id);
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /workbooks/{id}/restore`.
pub async fn restore(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
) -> Result<Json<Workbook>, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Manager).await?;
    let row = store::update(&state.pool, row.id, "trashed_at = NULL", None).await?;
    live::reopen(&state, row.id);
    let sheets = model::sheets(&state, row.id).await?;
    Ok(Json(row.into_workbook(sheets)))
}

/// `DELETE /workbooks/{id}/permanent`: only from the trash.
pub async fn delete_forever(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
) -> Result<StatusCode, Error> {
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Manager).await?;
    store::delete_trashed(&state.pool, row.id).await?;
    live::close(&state.hub, [row.id]);
    Ok(StatusCode::NO_CONTENT)
}
