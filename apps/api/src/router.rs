use std::path::PathBuf;

use axum::Router;
use axum::routing::get;
use tronc::health::{self, Check};
use tronc::http;

use crate::mcp::Mcp;
use crate::state::AppState;
use crate::{auth, modules, reference, security, version};

/// What the router needs besides the state.
#[derive(Clone, Default)]
pub struct Options {
    /// tronc's middleware chain.
    pub http: http::Config,
    /// The built SPA, served last when it exists.
    pub client_dir: PathBuf,
    /// `/mcp`, when `MCP_RESOURCE` is set.
    pub mcp: Option<Mcp>,
    /// Production: the client may not be framed from localhost.
    pub production: bool,
}

/// The `/api` routes: porte's, the nudges, then the modules. Unknown paths answer a JSON 404.
pub fn api(state: &AppState) -> Router<AppState> {
    let sessions = &state.kits.sessions;
    Router::new()
        .merge(auth::routes(sessions))
        .merge(sessions.routes())
        .merge(state.kits.oidc.routes())
        .merge(modules::spaces::routes(sessions))
        .merge(modules::workbooks::routes(sessions))
        .merge(modules::live::routes(sessions))
        .merge(modules::pending::routes(sessions))
        .route("/registre/sync", state.kits.syncer.nudge_route())
        .route("/registre/deletions", state.kits.deletions.route())
        .fallback(|| async { tronc::Error::not_found("no such route") })
        .layer(axum::middleware::from_fn_with_state(state.clone(), modules::live::on_logout))
}

/// The whole app: health, `/version`, `/api`, then the SPA.
pub fn build_router(mut state: AppState, options: Options) -> Result<Router, String> {
    if let Some(mcp) = &options.mcp {
        state.confirmations = Some(crate::mcp::confirmations(&state, &mcp.resource, &mcp.key)?);
    }
    let app = Router::new()
        .merge(health::routes(vec![Check::db(state.pool.clone())]))
        .route("/version", get(version::handler))
        .nest("/api", api(&state));
    let app = app.with_state(state.clone()).merge(reference::routes(options.mcp.is_some())?);
    let app = match options.mcp {
        Some(mcp) => crate::mcp::mount(app, &state, mcp)?,
        None => app,
    };
    let app = match tronc::spa::available(&options.client_dir) {
        true => app.fallback_service(security::spa(options.client_dir, options.production)?),
        false => app.fallback(http::not_found),
    };
    Ok(app.layer(http::layers(options.http)?))
}
