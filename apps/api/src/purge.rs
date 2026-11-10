use std::time::Duration;

use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;

use crate::modules::live;
use crate::modules::workbooks::store;
use crate::state::AppState;

/// How long a workbook stays in the trash.
pub const RETENTION: Duration = Duration::from_secs(30 * 24 * 3600);

/// How often the trash is emptied.
pub const INTERVAL: Duration = Duration::from_secs(3600);

/// Deletes the workbooks trashed more than [`RETENTION`] ago, and returns how many.
pub async fn purge_once(state: &AppState) -> Result<usize, tronc::Error> {
    let ids = store::purge(&state.pool, OffsetDateTime::now_utc() - RETENTION).await?;
    let count = ids.len();
    live::close(&state.hub, ids);
    Ok(count)
}

/// Runs [`purge_once`] at boot and every hour until `shutdown`. One replica runs it.
pub async fn run(state: AppState, shutdown: CancellationToken) {
    let mut ticker = tokio::time::interval(INTERVAL);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = ticker.tick() => {}
        }
        match purge_once(&state).await {
            Ok(0) => {}
            Ok(count) => tracing::info!(count, "trash: deleted workbooks older than 30 days"),
            Err(error) => tracing::warn!(error = %error.message, "trash: purge failed"),
        }
    }
}
