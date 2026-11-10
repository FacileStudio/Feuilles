use serde::{Deserialize, Serialize};
use tronc::mcp::{self, Answer, Call, Tool, ToolError};
use uuid::Uuid;

use super::{identity, spec};
use crate::engine::{CellWrite, SheetInfo};
use crate::hub::Change;
use crate::modules::workbooks::actions;
use crate::modules::workbooks::model;
use crate::modules::workbooks::service::{Access, require};
use crate::modules::workbooks::types::{CreateWorkbook, Workbook};
use crate::state::AppState;

/// `feuilles_write_cells` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// 1 to 5000 inputs, applied in order as one batch. A formula starts with `=`.
    pub writes: Vec<CellWrite>,
}

/// `feuilles_create_workbook` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    /// 1 to 200 characters.
    pub title: String,
    /// A space id from the person's spaces, or absent for a personal workbook.
    pub space_id: Option<String>,
}

/// `feuilles_add_sheet` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SheetInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The tab name, or absent for the next default name.
    pub name: Option<String>,
}

/// What a write answers.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WriteOutput {
    /// The batch's sequence number.
    pub seq: u64,
    /// The sheets after the write.
    pub sheets: Vec<SheetInfo>,
}

/// The tools that change workbooks. None deletes anything.
pub fn tools(state: &AppState) -> Vec<Tool> {
    let (one, two, three) = (state.clone(), state.clone(), state.clone());
    vec![
        mcp::tool(
            spec(
                "feuilles_write_cells",
                "Write cells",
                "Sets cell inputs, values or formulas, live for everyone.",
                false,
            ),
            move |call, input| write_cells(one.clone(), call, input),
        ),
        mcp::tool(
            spec("feuilles_create_workbook", "Create a workbook", "Creates an empty workbook with one sheet.", false),
            move |call, input| create(two.clone(), call, input),
        ),
        mcp::tool(
            spec("feuilles_add_sheet", "Add a sheet", "Adds a sheet at the end of a workbook.", false),
            move |call, input| add_sheet(three.clone(), call, input),
        ),
    ]
}

async fn change(state: &AppState, call: &Call, workbook: Uuid, changes: Vec<Change>) -> Result<WriteOutput, ToolError> {
    let me = identity(call)?;
    let (row, _) = require(state, &me, workbook, Access::Editor).await?;
    if row.trashed_at.is_some() {
        return Err(ToolError("the workbook is in the trash and read-only".into()));
    }
    let written = model::change(state, row.id, me.user_id, changes).await?;
    Ok(WriteOutput { seq: written.seq, sheets: written.sheets })
}

async fn write_cells(state: AppState, call: Call, input: WriteInput) -> Result<Answer<WriteOutput>, ToolError> {
    model::check_writes(&input.writes)?;
    let changes = input.writes.into_iter().map(Change::Cell).collect();
    Ok(change(&state, &call, input.workbook_id, changes).await?.into())
}

async fn create(state: AppState, call: Call, input: CreateInput) -> Result<Answer<Workbook>, ToolError> {
    let body =
        CreateWorkbook { title: input.title, space_id: input.space_id, locale: None, timezone: None, language: None };
    Ok(actions::create(&state, &identity(&call)?, &body).await?.into())
}

async fn add_sheet(state: AppState, call: Call, input: SheetInput) -> Result<Answer<WriteOutput>, ToolError> {
    Ok(change(&state, &call, input.workbook_id, vec![Change::AddSheet(input.name)]).await?.into())
}
