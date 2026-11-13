//! What the HTTP routes and the MCP tools both do.

use porte::Identity;
use tronc::Error;
use uuid::Uuid;

use super::service::{self, Access, require, space_scope};
use super::store::{self, Bind, NewRow, Row};
use super::types::{CreateWorkbook, Workbook};
use crate::engine;
use crate::modules::live;
use crate::state::AppState;

/// The workbooks of a space, or the caller's personal ones, newest change first.
pub async fn list(
    state: &AppState,
    identity: &Identity,
    space: Option<&str>,
    trashed: bool,
) -> Result<Vec<Workbook>, Error> {
    let space = space_scope(state, identity, space).await?;
    let rows = store::list(&state.pool, space.as_deref(), identity.user_id, trashed).await?;
    Ok(rows.into_iter().map(|row| row.into_workbook(Vec::new())).collect())
}

/// A new empty workbook in the caller's personal scope or a space they belong to.
pub async fn create(state: &AppState, identity: &Identity, body: &CreateWorkbook) -> Result<Workbook, Error> {
    let title = service::title(&body.title)?;
    let space = space_scope(state, identity, body.space_id.as_deref()).await?;
    let locale = body.locale.as_deref().unwrap_or("en");
    let timezone = body.timezone.as_deref().unwrap_or("UTC");
    let language = body.language.as_deref().unwrap_or("en");
    let model = engine::new_model(&title, locale, timezone, language)?;
    let new = NewRow {
        space_id: space.as_deref(),
        owner_id: Some(identity.user_id),
        title: &title,
        settings: (locale, timezone, language),
        model: &model,
    };
    let row = store::insert(&state.pool, new).await?;
    Ok(row.into_workbook(engine::sheets(&model)))
}

/// Renames a workbook the caller may edit.
pub async fn rename(state: &AppState, identity: &Identity, id: Uuid, title: &str) -> Result<Row, Error> {
    let title = service::title(title)?;
    let (row, _) = require(state, identity, id, Access::Editor).await?;
    store::update(&state.pool, row.id, "title = $2, updated_at = now()", Bind::Title(&title)).await
}

/// Moves a workbook the caller manages to a space they belong to, or to their personal
/// workbooks for `None`. Open sockets reconnect, so access is checked again.
pub async fn move_to(state: &AppState, identity: &Identity, id: Uuid, space: Option<&str>) -> Result<Row, Error> {
    let (row, _) = require(state, identity, id, Access::Manager).await?;
    let space = space_scope(state, identity, space).await?;
    if row.space_id == space {
        return Ok(row);
    }
    let set = "space_id = $2, owner_id = $3, updated_at = now()";
    let row = store::update(&state.pool, row.id, set, Bind::Scope(space.as_deref(), identity.user_id)).await?;
    live::reopen(state, row.id);
    Ok(row)
}

/// Moves a workbook the caller manages to the trash. Trashing it twice changes nothing.
pub async fn trash(state: &AppState, identity: &Identity, id: Uuid) -> Result<Row, Error> {
    let (row, _) = require(state, identity, id, Access::Manager).await?;
    let row = store::update(&state.pool, row.id, "trashed_at = COALESCE(trashed_at, now())", Bind::Nothing).await?;
    live::reopen(state, row.id);
    Ok(row)
}
