use std::collections::HashMap;

use serde::Serialize;
use tronc::Error;
use tronc::mcp::{ASK_AGAIN, Action, Answer, Call, Kind, Outcome, ToolError, failure};
use uuid::Uuid;

use super::planned::Planned;
use super::workbook;
use crate::engine::{SheetInfo, check_edit};
use crate::hub::Change;
use crate::modules::workbooks::model;
use crate::modules::workbooks::service::{Access, require};
use crate::state::AppState;

/// The tools that ask the person before they run: each removes cells for good.
pub const KINDS: [&str; 4] =
    ["feuilles_delete_sheet", "feuilles_delete_rows", "feuilles_delete_columns", "feuilles_clear_range"];

/// What a confirmed tool answers.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Confirmed {
    /// `done`, `declined` or `pending_confirmation`.
    pub status: String,
    /// The link the person opens to confirm, when the client could not ask.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub confirm_url: String,
    /// What happened and what to do next, for the model.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    /// The sheets after the edit, once it is done.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<SheetInfo>,
}

fn planned(action: &Action) -> Result<(i64, Planned), Error> {
    let target = serde_json::from_str(&action.target).map_err(|e| Error::internal("bad action target", e))?;
    let user = action.user_id.parse().map_err(|e| Error::internal("bad action user", e))?;
    Ok((user, target))
}

async fn editable(state: &AppState, user: i64, id: Uuid) -> Result<(), Error> {
    let me = porte::Identity { user_id: user, ..porte::Identity::default() };
    let (row, _) = require(state, &me, id, Access::Editor).await?;
    match row.trashed_at {
        Some(_) => Err(Error::failed("the workbook is in the trash and read-only; restore it in Feuilles first")),
        None => Ok(()),
    }
}

async fn fingerprint(state: AppState, action: Action) -> tronc::Result<String> {
    let (user, planned) = planned(&action)?;
    editable(&state, user, planned.workbook_id).await?;
    let seq: i64 = sqlx::query_scalar("SELECT seq FROM workbooks WHERE id = $1")
        .bind(planned.workbook_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::internal("read the workbook seq", e))?;
    Ok(seq.to_string())
}

async fn run(state: AppState, action: Action) -> tronc::Result<()> {
    let (user, planned) = planned(&action)?;
    editable(&state, user, planned.workbook_id).await?;
    let change = Change::Edit(planned.edit()?);
    model::change(&state, planned.workbook_id, user, vec![change]).await.map(drop)
}

/// The four kinds that ask before they run, each fingerprinted by the workbook's `seq`.
pub fn kinds(state: &AppState) -> HashMap<String, Kind> {
    HashMap::from(KINDS.map(|tool| {
        let (a, b) = (state.clone(), state.clone());
        let kind = Kind::new(tool, move |action| fingerprint(a.clone(), action), move |action| run(b.clone(), action));
        (tool.to_owned(), kind)
    }))
}

async fn answer(state: &AppState, outcome: Outcome) -> Result<Confirmed, ToolError> {
    let sheets = match (outcome.status, &outcome.action) {
        (tronc::mcp::STATUS_DONE, Some(action)) => model::sheets(state, planned(action)?.1.workbook_id).await?,
        _ => Vec::new(),
    };
    let (status, confirm_url, message) = (outcome.status.to_owned(), outcome.confirm_url, outcome.message);
    Ok(Confirmed { status, confirm_url, message, sheets })
}

/// Asks the person before `planned` runs as `kind`, after refusing what would fail anyway; on the
/// retry that carries their answer, runs it or drops it.
pub async fn ask(state: &AppState, call: &Call, kind: &str, planned: Planned) -> Result<Answer<Confirmed>, ToolError> {
    let confirmations = state.confirmations.clone().ok_or_else(|| ToolError::new("confirmations are off"))?;
    if call.retrying() {
        let outcome = confirmations.resume(call, kind).await.map_err(|e| failure(e, ASK_AGAIN))?;
        return Ok(answer(state, outcome).await?.into());
    }
    let (row, _) = workbook(state, call, planned.workbook_id, Access::Editor).await?;
    if row.trashed_at.is_some() {
        return Err(ToolError::new("the workbook is in the trash and read-only; restore it in Feuilles first"));
    }
    let names: Vec<String> = model::sheets(state, row.id).await?.into_iter().map(|sheet| sheet.name).collect();
    let sheet = check_edit(&names, &planned.edit()?)?;
    let question = planned.question(&row.title, &names[sheet as usize]);
    let target = serde_json::to_string(&planned).map_err(|e| ToolError(e.to_string()))?;
    let (asked, outcome) = confirmations.request(call, kind, &target, &question).await?;
    match asked {
        Some(question) => Ok(Answer::InputRequired(question)),
        None => Ok(answer(state, outcome).await?.into()),
    }
}
