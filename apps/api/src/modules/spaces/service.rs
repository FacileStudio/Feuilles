use porte::Identity;
use tronc::Error;

use super::types::SpaceItem;
use crate::modules::workbooks::service::caller;
use crate::state::AppState;

/// The spaces the caller is a member of, by name.
pub async fn mine(state: &AppState, identity: &Identity) -> Result<Vec<SpaceItem>, Error> {
    let Some(subject) = caller(state, identity).await?.subject else {
        return Ok(Vec::new());
    };
    sqlx::query_as::<_, SpaceItem>(
        "SELECT s.id, s.name, m.role FROM space_members m JOIN spaces s ON s.id = m.space_id
         WHERE m.subject = $1 ORDER BY lower(s.name), s.id",
    )
    .bind(subject)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::internal("list spaces", e))
}
