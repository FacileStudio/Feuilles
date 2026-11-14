//! `/mcp`: curated workbook, sheet and cell tools on `tronc::mcp`, with tokens checked by
//! `porte::mcpauth`. Free on every plan. The tools that remove cells for good ask the person
//! first.

mod confirm;
mod lines;
mod planned;
mod read;
mod sheets;
mod workbooks;
mod write;

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use porte::{Identity, mcpauth};
use tronc::mcp::{Annotations, App, AppConfig, AppSpec, Call, PostgresPending, ToolError, ToolSpec};
use uuid::Uuid;

use crate::auth::Kits;
use crate::modules::workbooks::service::{Access, require};
use crate::modules::workbooks::store::Row;
use crate::state::AppState;

pub use confirm::KINDS;

/// The key that signs the confirmation questions, from `secret`, `PORTE_TOKEN_KEY`.
pub fn state_key(secret: &str) -> Vec<u8> {
    tronc::mcp::state_key("feuilles", secret)
}

/// The Feuilles MCP server over `cfg`, with its confirmations on `mcp_actions`. Its tools are
/// added by [`mount`], once the state holds [`App::confirmations`].
pub fn app(state: &AppState, mut cfg: AppConfig) -> Result<App, String> {
    cfg.version = crate::version::VERSION.into();
    cfg.pending = Some(Arc::new(PostgresPending::new(state.pool.clone())));
    let spec = AppSpec {
        name: "feuilles".into(),
        title: "Feuilles".into(),
        instructions: "Spreadsheets of the Facile suite. List workbooks, read ranges, write cells, and \
            change sheets, rows and columns. Rows and columns are numbered from 1; a sheet is its 0-based index \
            or its tab name. Every write reaches the people who have the workbook open."
            .into(),
        kinds: confirm::kinds(state),
        ..AppSpec::default()
    };
    App::new(cfg, spec)
}

/// Mounts `/mcp` and its protected-resource metadata on the root router, with every tool.
pub fn mount(router: Router, state: &AppState, mut app: App) -> Result<Router, String> {
    app.options.tools =
        [read::tools(state), write::tools(state), workbooks::tools(state), sheets::tools(state), lines::tools(state)]
            .concat();
    app.mount(router)
}

/// The verifier for `resource`: Registre's tokens, accounts provisioned on a first call.
/// porte's verifier implements `tronc::mcp::Verifier` itself.
pub async fn porte_verifier(
    cfg: &porte::Config,
    kits: &Kits,
    resource: &str,
) -> Result<Arc<mcpauth::Verifier>, porte::ConfigError> {
    let verifier = mcpauth::Verifier::new(mcpauth::Config {
        issuer: cfg.issuer.clone(),
        resource: resource.to_owned(),
        scope_prefix: "feuilles".into(),
        audience: cfg.client_id.clone(),
        identities: kits.store.identities(),
        provision: true,
        accounts: Some(kits.oidc.clone()),
        http: porte::http_client::HttpClient::new(),
    })
    .await?;
    Ok(Arc::new(verifier))
}

fn spec(name: &str, title: &str, description: &str, annotations: Annotations) -> ToolSpec {
    ToolSpec { name: name.into(), title: title.into(), description: description.into(), annotations: Some(annotations) }
}

fn with<In: 'static, Fut: Future + 'static>(
    state: &AppState,
    handler: fn(AppState, Call, In) -> Fut,
) -> impl Fn(Call, In) -> Fut + Send + Sync + 'static {
    let state = state.clone();
    move |call, input| handler(state.clone(), call, input)
}

async fn workbook(state: &AppState, call: &Call, id: Uuid, min: Access) -> Result<(Row, Identity), ToolError> {
    let me = identity(call)?;
    let hint = "Call feuilles_list_workbooks for the ids you can open.";
    let (row, _) = require(state, &me, id, min).await.map_err(|mut error| {
        if error.code == tronc::Code::NotFound {
            error.message = format!("workbook {id} not found");
        }
        tronc::mcp::failure(error, hint)
    })?;
    Ok((row, me))
}

fn identity(call: &Call) -> Result<Identity, ToolError> {
    let user_id = call.caller.user_id.parse().map_err(|_| ToolError("the token names no Feuilles account".into()))?;
    Ok(Identity {
        user_id,
        actor: call.caller.actor.clone(),
        scopes: call.caller.scopes.clone(),
        ..Identity::default()
    })
}
