use serde::{Deserialize, Serialize};
use tronc::mcp::{self, Answer, Call, Tool, ToolError};
use uuid::Uuid;

use super::{READ, identity, spec, with, workbook};
use crate::engine::{Area, Cell, SheetInfo, SheetRef, parse_range, sheet_index};
use crate::modules::workbooks::actions;
use crate::modules::workbooks::model::{self, MAX_READ_CELLS};
use crate::modules::workbooks::service::Access;
use crate::modules::workbooks::types::Workbook;
use crate::state::AppState;

const DEFAULT_LIST: usize = 50;
const MAX_LIST: usize = 200;

/// `feuilles_list_workbooks` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {
    /// A space id from the person's spaces, or absent for their personal workbooks.
    pub space_id: Option<String>,
    /// `next_cursor` from the previous page.
    pub cursor: Option<String>,
    /// 1 to 200 workbooks per page. Default 50.
    pub limit: Option<usize>,
}

/// `feuilles_list_workbooks` output.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListOutput {
    /// Newest change first. `sheets` is empty here: get one workbook for its sheets.
    pub workbooks: Vec<Workbook>,
    /// Pass it back as `cursor` for the next workbooks, absent on the last page.
    pub next_cursor: Option<String>,
}

/// `feuilles_get_workbook` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetInput {
    /// The workbook id.
    pub workbook_id: Uuid,
}

/// `feuilles_read_range` input.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RangeInput {
    /// The workbook id.
    pub workbook_id: Uuid,
    /// The sheet: its index from 0, or its tab name. Default 0.
    pub sheet: Option<SheetRef>,
    /// Such as `A1:D20`. Rows and columns past the sheet's data are left out.
    pub range: String,
    /// `next_cursor` from the previous page.
    pub cursor: Option<String>,
}

/// `feuilles_read_range` output.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct RangeOutput {
    /// The sheet index.
    pub sheet: u32,
    /// The cells this page holds, such as `A1:D40`.
    pub range: String,
    /// The cells, row by row: input, value, and the value as displayed.
    pub rows: Vec<Vec<Cell>>,
    /// Pass it back as `cursor` for the next rows, absent on the last page.
    pub next_cursor: Option<String>,
    /// How to read the rest, when this page is not the last.
    pub hint: Option<String>,
}

/// The read-only tools.
pub fn tools(state: &AppState) -> Vec<Tool> {
    vec![
        mcp::tool(
            spec(
                "feuilles_list_workbooks",
                "List workbooks",
                "Call first to find a workbook id. Lists the workbooks of a space, or the person's own, by pages.",
                READ,
            ),
            with(state, list),
        ),
        mcp::tool(
            spec(
                "feuilles_get_workbook",
                "Get a workbook",
                "Call before reading or editing a workbook: its title, sheets, and the used range of each sheet.",
                READ,
            ),
            with(state, get),
        ),
        mcp::tool(
            spec(
                "feuilles_read_range",
                "Read a range",
                "Call to read cells: inputs, values and displayed values of a range, by pages of rows.",
                READ,
            ),
            with(state, read_range),
        ),
    ]
}

async fn list(state: AppState, call: Call, input: ListInput) -> Result<Answer<ListOutput>, ToolError> {
    let limit = input.limit.unwrap_or(DEFAULT_LIST);
    if !(1..=MAX_LIST).contains(&limit) {
        return Err(ToolError::new(format!("limit must be 1 to {MAX_LIST}")));
    }
    let start = mcp::decode_cursor::<usize>(input.cursor.as_deref().unwrap_or_default())?.unwrap_or(0);
    let all = actions::list(&state, &identity(&call)?, input.space_id.as_deref(), false).await?;
    let mut workbooks: Vec<Workbook> = all.into_iter().skip(start).collect();
    let more = workbooks.len();
    workbooks.truncate(limit);
    let fitted = mcp::fit_count(workbooks.len(), |n| mcp::encoded_size(&workbooks[..n])).max(1).min(workbooks.len());
    workbooks.truncate(fitted);
    let next_cursor = (fitted < more).then(|| mcp::encode_cursor(&(start + fitted)));
    Ok(ListOutput { workbooks, next_cursor }.into())
}

async fn get(state: AppState, call: Call, input: GetInput) -> Result<Answer<Workbook>, ToolError> {
    let (row, _) = workbook(&state, &call, input.workbook_id, Access::Viewer).await?;
    let sheets = model::sheets(&state, row.id).await?;
    Ok(row.into_workbook(sheets).into())
}

async fn read_range(state: AppState, call: Call, input: RangeInput) -> Result<Answer<RangeOutput>, ToolError> {
    let asked = parse_range(&input.range)?;
    let (row, _) = workbook(&state, &call, input.workbook_id, Access::Viewer).await?;
    let sheets = model::sheets(&state, row.id).await?;
    let names: Vec<String> = sheets.iter().map(|s| s.name.clone()).collect();
    let sheet = sheet_index(&names, &input.sheet.unwrap_or_default())?;
    let used = sheets.iter().find(|s| s.index == sheet).and_then(|s| s.used_range.clone());
    let area = within(asked, used.as_deref());
    let width = i64::from(area.right - area.left + 1);
    if width > MAX_READ_CELLS {
        return Err(ToolError::new(too_wide(width, used.as_deref())));
    }
    let start = mcp::decode_cursor::<i32>(input.cursor.as_deref().unwrap_or_default())?.unwrap_or(area.top);
    if !(area.top..=area.bottom).contains(&start) {
        return Err(ToolError::new(mcp::INVALID_CURSOR));
    }
    let last = area.bottom.min(start + i32::try_from(MAX_READ_CELLS / width).unwrap_or(1) - 1);
    let mut rows = model::read(&state, row.id, sheet, Area { top: start, bottom: last, ..area }).await?;
    let fitted = mcp::fit_count(rows.len(), |n| mcp::encoded_size(&rows[..n])).max(1);
    rows.truncate(fitted);
    let end = start + i32::try_from(fitted).unwrap_or(1) - 1;
    let next_cursor = (end < area.bottom).then(|| mcp::encode_cursor(&(end + 1)));
    let hint = next_cursor.as_ref().map(|_| more_rows(end + 1, area, &sheets[sheet as usize]));
    let range = Area { top: start, bottom: end, ..area }.label();
    Ok(RangeOutput { sheet, range, rows, next_cursor, hint }.into())
}

fn within(asked: Area, used: Option<&str>) -> Area {
    let Some(used) = used.and_then(|used| parse_range(used).ok()) else {
        return Area { bottom: asked.top, right: asked.left, ..asked };
    };
    Area {
        bottom: asked.bottom.min(used.bottom.max(asked.top)),
        right: asked.right.min(used.right.max(asked.left)),
        ..asked
    }
}

fn too_wide(width: i64, used: Option<&str>) -> String {
    let used = used.unwrap_or("empty");
    format!(
        "the range is {width} columns wide; a read covers at most {MAX_READ_CELLS} cells per row, so narrow the columns (the sheet's used range is {used})"
    )
}

fn more_rows(next: i32, area: Area, sheet: &SheetInfo) -> String {
    let rest = Area { top: next, ..area }.label();
    let used = sheet.used_range.as_deref().unwrap_or("empty");
    format!(
        "rows {rest} remain: pass next_cursor to read them, or ask for a narrower range (the used range of '{}' is {used})",
        sheet.name
    )
}
