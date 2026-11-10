use axum::extract::{Path, Query, State};
use porte::Identity;
use tronc::Error;
use tronc::json::{Json, JsonLimit};

use super::model;
use super::service::{Access, parse_id, require};
use super::types::{CellsAnswer, CellsQuery, WriteAnswer, WriteCells};
use crate::engine::parse_range;
use crate::state::AppState;

/// The largest `PUT /cells` body.
pub const MAX_WRITE_BODY: usize = 4 << 20;

/// `GET /workbooks/{id}/cells?sheet=0&range=A1:D20`.
pub async fn read(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
    Query(query): Query<CellsQuery>,
) -> Result<Json<CellsAnswer>, Error> {
    let area = parse_range(&query.range)?;
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Viewer).await?;
    let rows = model::read(&state, row.id, query.sheet, area).await?;
    Ok(Json(CellsAnswer { sheet: query.sheet, range: area.label(), rows }))
}

/// `PUT /workbooks/{id}/cells`: 1 to 5000 inputs, as one batch.
pub async fn write(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<String>,
    JsonLimit(body): JsonLimit<WriteCells, MAX_WRITE_BODY>,
) -> Result<Json<WriteAnswer>, Error> {
    model::check_writes(&body.writes)?;
    let (row, _) = require(&state, &identity, parse_id(&id)?, Access::Editor).await?;
    if row.trashed_at.is_some() {
        return Err(Error::failed("a workbook in the trash is read-only"));
    }
    let seq = model::write(&state, row.id, identity.user_id, &body.writes).await?;
    Ok(Json(WriteAnswer { seq }))
}
