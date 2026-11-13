//! `/mcp`: curated workbook, sheet and cell tools on `tronc::mcp`, with tokens checked by
//! `porte::mcpauth`. Free on every plan.

mod lines;
mod read;
mod sheets;
mod workbooks;
mod write;

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use porte::{Identity, mcpauth};
use tronc::mcp::{Annotations, Call, Options, ToolError, ToolSpec};
use uuid::Uuid;

use crate::auth::Kits;
use crate::modules::workbooks::service::{Access, require};
use crate::modules::workbooks::store::Row;
use crate::state::AppState;

/// A read: changes nothing, safe to repeat.
const READ: Annotations = Annotations { read_only: true, destructive: false, idempotent: true, open_world: false };
/// Adds something new each call and overwrites nothing.
const ADD: Annotations = Annotations { read_only: false, destructive: false, idempotent: false, open_world: false };
/// Rearranges without losing anything, and the same call again changes nothing more.
const ARRANGE: Annotations = Annotations { read_only: false, destructive: false, idempotent: true, open_world: false };
/// Overwrites or removes, and the same call again changes nothing more.
const OVERWRITE: Annotations = Annotations { read_only: false, destructive: true, idempotent: true, open_world: false };
/// Removes or shifts content, and the same call again does it again.
const REMOVE: Annotations = Annotations { read_only: false, destructive: true, idempotent: false, open_world: false };

/// What mounting `/mcp` needs.
#[derive(Clone)]
pub struct Mcp {
    /// `MCP_RESOURCE`, the public URL of `/mcp`.
    pub resource: String,
    /// The authorization server, `OIDC_ISSUER`.
    pub issuer: String,
    /// Checks bearer tokens.
    pub verifier: Arc<dyn tronc::mcp::Verifier>,
}

/// Mounts `/mcp` and its protected-resource metadata on the root router.
pub fn mount(app: Router, state: &AppState, mcp: Mcp) -> Result<Router, String> {
    let mut options = Options::new("feuilles", mcp.resource, mcp.issuer, mcp.verifier);
    options.title = "Feuilles".into();
    options.version = crate::version::VERSION.into();
    options.instructions = "Spreadsheets of the Facile suite. List workbooks, read ranges, write cells, and \
        change sheets, rows and columns. Rows and columns are numbered from 1; a sheet is its 0-based index \
        or its tab name. Every write reaches the people who have the workbook open."
        .into();
    options.tools =
        [read::tools(state), write::tools(state), workbooks::tools(state), sheets::tools(state), lines::tools(state)]
            .concat();
    tronc::mcp::mount(app, options)
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
