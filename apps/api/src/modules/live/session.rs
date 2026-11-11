use sqlx::PgPool;

use super::socket::{Flow, codes};
use crate::hub::Hub;

/// The close reason for a socket whose session ended.
pub const ENDED: &str = "unauthenticated";

/// Whether session `id` still exists and has not expired. A bearer JWT has no row (`id` 0): its
/// expiry was checked at the upgrade. A database error keeps the socket: the next check retries.
pub async fn alive(pool: &PgPool, id: i64) -> bool {
    if id == 0 {
        return true;
    }
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM porte_sessions WHERE id = $1 AND (expires_at IS NULL OR expires_at > now()))",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap_or(true)
}

/// [`Flow::Close`] with 1008 when the socket's session ended, [`Flow::Continue`] otherwise.
pub(super) async fn check(hub: &Hub, id: i64) -> Flow {
    match alive(&hub.sessions.pool, id).await {
        true => Flow::Continue,
        false => Flow::Close(codes::POLICY, ENDED.into()),
    }
}
