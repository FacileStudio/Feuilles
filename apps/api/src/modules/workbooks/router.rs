use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post};
use porte::session::Manager;

use super::imports::MAX_IMPORT_BYTES;
use super::{cells, files, handlers};
use crate::state::AppState;

/// The workbook routes, behind the session.
pub fn routes(sessions: &Manager) -> Router<AppState> {
    Router::new()
        .route("/workbooks", get(handlers::list).post(handlers::create))
        .route("/workbooks/import", post(files::import).layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES + (64 << 10))))
        .route("/workbooks/{id}", get(handlers::get).patch(handlers::rename).delete(handlers::trash))
        .route("/workbooks/{id}/restore", post(handlers::restore))
        .route("/workbooks/{id}/permanent", delete(handlers::delete_forever))
        .route("/workbooks/{id}/export.xlsx", get(files::export_xlsx))
        .route("/workbooks/{id}/export.csv", get(files::export_csv))
        .route("/workbooks/{id}/cells", get(cells::read).put(cells::write))
        .route_layer(sessions.require_auth())
}
