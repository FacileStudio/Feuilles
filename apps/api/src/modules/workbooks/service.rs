//! Who may do what to a workbook. Every workbook route goes through [`require`].

use porte::Identity;
use porte::spaces::SpaceError;
use tronc::Error;
use uuid::Uuid;

use super::store::{self, Row};
use crate::auth::subject_of;
use crate::state::AppState;

/// What a caller may do to one workbook, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    /// Nothing: the workbook answers 404, as if it did not exist.
    None,
    /// Read it.
    Viewer,
    /// Read it, edit cells and rename it.
    Editor,
    /// All of the above, plus trash, restore and delete.
    Manager,
}

/// The caller as the access rules see them.
#[derive(Debug, Clone)]
pub struct Caller {
    /// `porte_users.id`.
    pub user_id: i64,
    /// Their Registre subject, the key of their space memberships.
    pub subject: Option<String>,
}

/// Resolves the caller's Registre subject.
pub async fn caller(state: &AppState, identity: &Identity) -> Result<Caller, Error> {
    let subject = match identity.subject.is_empty() {
        true => subject_of(&state.pool, &state.issuer, identity.user_id).await?,
        false => Some(identity.subject.clone()),
    };
    Ok(Caller { user_id: identity.user_id, subject })
}

/// What `identity` may do to `workbook`. A personal workbook belongs to its owner alone. In a
/// space, a member edits and an admin or owner manages. An instance admin gets no bypass.
pub async fn access(state: &AppState, identity: &Identity, workbook: Uuid) -> Result<Access, Error> {
    let caller = caller(state, identity).await?;
    match store::find(&state.pool, workbook).await? {
        Some(row) => access_to(state, &caller, &row).await,
        None => Ok(Access::None),
    }
}

async fn access_to(state: &AppState, caller: &Caller, row: &Row) -> Result<Access, Error> {
    let Some(space) = &row.space_id else {
        return Ok(if row.owner_id == Some(caller.user_id) { Access::Manager } else { Access::None });
    };
    Ok(match role_in(state, caller, space).await?.as_deref() {
        Some("member") => Access::Editor,
        Some("admin" | "owner") => Access::Manager,
        _ => Access::None,
    })
}

async fn role_in(state: &AppState, caller: &Caller, space: &str) -> Result<Option<String>, Error> {
    let Some(subject) = &caller.subject else {
        return Ok(None);
    };
    match state.guard.resolve(subject, space).await {
        Ok(scope) => Ok(Some(scope.role.as_str().to_owned())),
        Err(SpaceError::Store(cause)) => Err(Error::internal("read a space membership", cause)),
        Err(_) => Ok(None),
    }
}

/// The workbook and the caller's access to it, when that is at least `min`. No access is 404;
/// too little is 403.
pub async fn require(state: &AppState, identity: &Identity, id: Uuid, min: Access) -> Result<(Row, Access), Error> {
    let caller = caller(state, identity).await?;
    let row = store::find(&state.pool, id).await?.ok_or_else(|| Error::not_found("not found"))?;
    match access_to(state, &caller, &row).await? {
        Access::None => Err(Error::not_found("not found")),
        access if access < min => Err(Error::forbidden("your role in this space does not allow this")),
        access => Ok((row, access)),
    }
}

/// The scope of a list or a create: `Some(space)` when the caller is a member, `None` for
/// personal scope, 404 for a space they are not in.
pub async fn space_scope(state: &AppState, identity: &Identity, space: Option<&str>) -> Result<Option<String>, Error> {
    let Some(space) = space.map(str::trim).filter(|space| !space.is_empty()) else {
        return Ok(None);
    };
    let caller = caller(state, identity).await?;
    match role_in(state, &caller, space).await? {
        Some(_) => Ok(Some(space.to_owned())),
        None => Err(Error::not_found("not found")),
    }
}

/// A title trimmed to 1 to 200 characters, or 400.
pub fn title(raw: &str) -> Result<String, Error> {
    let trimmed = raw.trim();
    match trimmed.chars().count() {
        1..=200 => Ok(trimmed.to_owned()),
        _ => Err(Error::invalid("title must be 1 to 200 characters")),
    }
}

/// A workbook id from the path. Anything that is not a UUID is 404, like an unknown id.
pub fn parse_id(raw: &str) -> Result<Uuid, Error> {
    Uuid::parse_str(raw).map_err(|_| Error::not_found("not found"))
}
