use serde::Deserialize;
use tronc::mcp::{self, Annotations, Answer, Call, Tool, ToolError};
use uuid::Uuid;

use super::confirm::{Confirmed, ask};
use super::planned::{Op, Planned};
use super::write::{WriteOutput, change};
use super::{spec, with};
use crate::engine::{Edit, EditKind, SheetRef};
use crate::hub::Change;
use crate::state::AppState;

/// `feuilles_update_sheet` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The sheet: its index from 0, or its tab name.
    pub sheet: SheetRef,
    /// A new tab name, or absent to keep it.
    pub name: Option<String>,
    /// The 0-based position to move the sheet to, or absent to leave it in place.
    pub position: Option<u32>,
}

/// `feuilles_delete_sheet` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SheetInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The sheet: its index from 0, or its tab name.
    pub sheet: SheetRef,
}

/// `feuilles_clear_range` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClearInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The sheet: its index from 0, or its tab name.
    pub sheet: SheetRef,
    /// Such as `A1:D20`.
    pub range: String,
}

/// The tools that rename, reorder and delete sheets and clear ranges.
pub fn tools(state: &AppState) -> Vec<Tool> {
    vec![
        mcp::tool(
            spec(
                "feuilles_update_sheet",
                "Rename or move a sheet",
                "Call to rename a sheet or change its tab order.",
                Annotations::OVERWRITE,
            ),
            with(state, update),
        ),
        mcp::tool(
            spec(
                "feuilles_delete_sheet",
                "Delete a sheet",
                "Call only when the person asks to delete a sheet and its cells. A workbook keeps at least one sheet. The person confirms first.",
                Annotations::REMOVE,
            ),
            with(state, delete),
        ),
        mcp::tool(
            spec(
                "feuilles_clear_range",
                "Clear a range",
                "Call to empty the cells of a range. Their formatting stays; the rows and columns do not move. The person confirms first.",
                Annotations::OVERWRITE,
            ),
            with(state, clear),
        ),
    ]
}

async fn update(state: AppState, call: Call, input: UpdateInput) -> Result<Answer<WriteOutput>, ToolError> {
    let mut edits = Vec::new();
    let mut sheet = input.sheet;
    if let Some(to) = input.position {
        edits.push(Edit { sheet, kind: EditKind::Move(to) });
        sheet = SheetRef::Index(to);
    }
    if let Some(name) = input.name {
        edits.push(Edit { sheet, kind: EditKind::Rename(name) });
    }
    if edits.is_empty() {
        return Err(ToolError::new("give a name, a position, or both"));
    }
    let changes = edits.into_iter().map(Change::Edit).collect();
    Ok(change(&state, &call, input.workbook_id, changes).await?.into())
}

async fn delete(state: AppState, call: Call, input: SheetInput) -> Result<Answer<Confirmed>, ToolError> {
    let planned = Planned { workbook_id: input.workbook_id, sheet: input.sheet, op: Op::DeleteSheet };
    ask(&state, &call, "feuilles_delete_sheet", planned).await
}

async fn clear(state: AppState, call: Call, input: ClearInput) -> Result<Answer<Confirmed>, ToolError> {
    let planned = Planned { workbook_id: input.workbook_id, sheet: input.sheet, op: Op::Clear { range: input.range } };
    ask(&state, &call, "feuilles_clear_range", planned).await
}
