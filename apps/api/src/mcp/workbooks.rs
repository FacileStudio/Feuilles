use serde::Deserialize;
use tronc::mcp::{self, Annotations, Answer, Call, Tool, ToolError};
use uuid::Uuid;

use super::{spec, with, workbook};
use crate::modules::workbooks::actions;
use crate::modules::workbooks::model;
use crate::modules::workbooks::service::Access;
use crate::modules::workbooks::types::Workbook;
use crate::state::AppState;

/// `feuilles_update_workbook` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// A new title, 1 to 200 characters, or absent to keep it.
    pub title: Option<String>,
    /// Moves the workbook: a space id from the person's spaces, `""` for their personal
    /// workbooks, or absent to leave it where it is. Needs the manager role.
    pub space_id: Option<String>,
}

/// `feuilles_delete_workbook` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteInput {
    /// The workbook id.
    pub workbook_id: Uuid,
}

/// The tools that rename, move and trash whole workbooks.
pub fn tools(state: &AppState) -> Vec<Tool> {
    vec![
        mcp::tool(
            spec(
                "feuilles_update_workbook",
                "Rename or move a workbook",
                "Call to rename a workbook or move it to another space. Moving it changes who can open it.",
                Annotations::OVERWRITE,
            ),
            with(state, update),
        ),
        mcp::tool(
            spec(
                "feuilles_delete_workbook",
                "Delete a workbook",
                "Call only when the person asks to delete a workbook. It goes to the trash, closes for everyone, \
                 and is erased for good after 30 days; it can be restored from the trash in Feuilles until then.",
                Annotations::OVERWRITE,
            ),
            with(state, delete),
        ),
    ]
}

async fn update(state: AppState, call: Call, input: UpdateInput) -> Result<Answer<Workbook>, ToolError> {
    if input.title.is_none() && input.space_id.is_none() {
        return Err(ToolError::new("give a title, a space_id, or both"));
    }
    let (mut row, me) = workbook(&state, &call, input.workbook_id, Access::Editor).await?;
    if let Some(title) = &input.title {
        row = actions::rename(&state, &me, row.id, title).await?;
    }
    if let Some(space) = &input.space_id {
        let hint = "Space ids come from the space_id of a listed workbook or from the person.";
        let moved = actions::move_to(&state, &me, row.id, Some(space)).await.map_err(|mut error| {
            if error.code == tronc::Code::NotFound {
                error.message = format!("space '{space}' not found among your spaces");
            }
            mcp::failure(error, hint)
        })?;
        row = moved;
    }
    let sheets = model::sheets(&state, row.id).await?;
    Ok(row.into_workbook(sheets).into())
}

async fn delete(state: AppState, call: Call, input: DeleteInput) -> Result<Answer<Workbook>, ToolError> {
    let (row, me) = workbook(&state, &call, input.workbook_id, Access::Manager).await?;
    let row = actions::trash(&state, &me, row.id).await?;
    Ok(row.into_workbook(Vec::new()).into())
}
