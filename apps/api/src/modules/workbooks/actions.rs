//! What the HTTP routes and the MCP tools both do.

use porte::Identity;
use tronc::Error;

use super::service::{self, space_scope};
use super::store::{self, NewRow};
use super::types::{CreateWorkbook, Workbook};
use crate::engine;
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
        owner_id: identity.user_id,
        title: &title,
        settings: (locale, timezone, language),
        snapshot: &model.to_bytes(),
    };
    let row = store::insert(&state.pool, new).await?;
    Ok(row.into_workbook(engine::sheets(&model)))
}
