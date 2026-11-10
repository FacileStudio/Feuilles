use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use porte::Identity;
use tronc::Error;
use tronc::json::Json;

use super::model;
use super::service::{self, Access, parse_id, require, space_scope};
use super::store::{self, NewRow};
use super::types::{CreateWorkbook, ListQuery, RenameWorkbook, Workbook, WorkbookList};
use crate::engine;
use crate::state::AppState;

/// `GET /workbooks`.
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
    Query(query): Query<ListQuery>,
) -> Result<Json<WorkbookList>, Error> {
    let space = space_scope(&state, &identity, query.space_id.as_deref()).await?;
    let rows = store::list(&state.pool, space.as_deref(), identity.user_id, query.trashed).await?;
    Ok(Json(WorkbookList { workbooks: rows.into_iter().map(|row| row.into_workbook(Vec::new())).collect() }))
}

/// `POST /workbooks`.
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    Json(body): Json<CreateWorkbook>,
) -> Result<Response, Error> {
    let title = service::title(&body.title)?;
    let space = space_scope(&state, &identity, body.space_id.as_deref()).await?;
    let locale = body.locale.as_deref().unwrap_or("en");
    let timezone = body.timezone.as_deref().unwrap_or("UTC");
    let language = body.language.as_deref().unwrap_or("en");
    let model = engine::new_model(&title, locale, timezone, language)?;
    let new = NewRow {
        space_id: space.as_deref(),
        owner_id: identity.user_id,
        title: &title,
        settings: (locale, timezone, language),
        snapshot: &model.to_bytes(),
    };
    let row = store::insert(&state.pool, new).await?;
    Ok((StatusCode::CREATED, Json(row.into_workbook(engine::sheets(&model)))).into_response())
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
    Ok(StatusCode::NO_CONTENT)
}
