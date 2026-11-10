//! `/mcp`: six curated tools on `tronc::mcp`, with tokens checked by `porte::mcpauth`. Free on
//! every plan.

mod read;
mod verifier;
mod write;

use std::sync::Arc;

use axum::Router;
use porte::Identity;
use tronc::mcp::{Annotations, Call, Options, ToolError, ToolSpec};

pub use verifier::{PorteVerifier, porte_verifier};

use crate::state::AppState;

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
    options.instructions = "Spreadsheets of the Facile suite. List workbooks, read ranges, write cells. \
        Cell addresses are 1-based rows and columns; sheets are 0-based."
        .into();
    options.tools = [read::tools(state), write::tools(state)].concat();
    tronc::mcp::mount(app, options)
}

fn spec(name: &str, title: &str, description: &str, read_only: bool) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        title: title.into(),
        description: description.into(),
        annotations: Some(Annotations { read_only, destructive: false, idempotent: read_only, open_world: false }),
    }
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
