use serde::Deserialize;
use tronc::mcp::{self, Answer, Call, Tool, ToolError};
use uuid::Uuid;

use super::confirm::{Confirmed, ask};
use super::planned::{Op, Planned};
use super::write::{WriteOutput, change};
use super::{ADD, REMOVE, spec, with};
use crate::engine::{Axis, Edit, EditKind, SheetRef};
use crate::hub::Change;
use crate::state::AppState;

/// The input of the row and column tools.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinesInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The sheet: its index from 0, or its tab name.
    pub sheet: SheetRef,
    /// The first row or column, from 1 (column `A` is 1).
    pub at: i32,
    /// How many, 1 to 10000. Default 1.
    pub count: Option<i32>,
}

/// The tools that insert and delete rows and columns.
pub fn tools(state: &AppState) -> Vec<Tool> {
    let insert = "Formulas follow the cells they point to.";
    let delete = "Call only when the person asks to remove them with their cells; formulas pointing there break. The person confirms first.";
    vec![
        mcp::tool(
            spec(
                "feuilles_insert_rows",
                "Insert rows",
                &format!("Call to insert blank rows before `at`. {insert}"),
                ADD,
            ),
            with(state, insert_rows),
        ),
        mcp::tool(
            spec("feuilles_delete_rows", "Delete rows", &format!("Deletes rows from `at`. {delete}"), REMOVE),
            with(state, delete_rows),
        ),
        mcp::tool(
            spec(
                "feuilles_insert_columns",
                "Insert columns",
                &format!("Call to insert blank columns before `at`. {insert}"),
                ADD,
            ),
            with(state, insert_columns),
        ),
        mcp::tool(
            spec("feuilles_delete_columns", "Delete columns", &format!("Deletes columns from `at`. {delete}"), REMOVE),
            with(state, delete_columns),
        ),
    ]
}

async fn lines(state: &AppState, call: &Call, input: LinesInput, kind: EditKind) -> Result<WriteOutput, ToolError> {
    let edit = Edit { sheet: input.sheet, kind };
    change(state, call, input.workbook_id, vec![Change::Edit(edit)]).await
}

async fn insert_rows(state: AppState, call: Call, input: LinesInput) -> Result<Answer<WriteOutput>, ToolError> {
    let kind = EditKind::Insert { axis: Axis::Rows, at: input.at, count: input.count.unwrap_or(1) };
    Ok(lines(&state, &call, input, kind).await?.into())
}

async fn delete_rows(state: AppState, call: Call, input: LinesInput) -> Result<Answer<Confirmed>, ToolError> {
    let op = Op::DeleteRows { at: input.at, count: input.count.unwrap_or(1) };
    let planned = Planned { workbook_id: input.workbook_id, sheet: input.sheet, op };
    ask(&state, &call, "feuilles_delete_rows", planned).await
}

async fn insert_columns(state: AppState, call: Call, input: LinesInput) -> Result<Answer<WriteOutput>, ToolError> {
    let kind = EditKind::Insert { axis: Axis::Columns, at: input.at, count: input.count.unwrap_or(1) };
    Ok(lines(&state, &call, input, kind).await?.into())
}

async fn delete_columns(state: AppState, call: Call, input: LinesInput) -> Result<Answer<Confirmed>, ToolError> {
    let op = Op::DeleteColumns { at: input.at, count: input.count.unwrap_or(1) };
    let planned = Planned { workbook_id: input.workbook_id, sheet: input.sheet, op };
    ask(&state, &call, "feuilles_delete_columns", planned).await
}
