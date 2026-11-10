//! The API reference at `/docs`, from a hand-kept table checked against the router by a test.

mod auth;
mod table;
mod workbooks;

use axum::Router;
use tronc::apiref::{self, Config};
use utoipa::openapi::{OpenApi, OpenApiBuilder, PathsBuilder};

pub use table::Route;

/// The reference's title, version and servers.
pub fn config() -> Config {
    Config {
        title: "Feuilles API".into(),
        version: crate::version::VERSION.trim_start_matches('v').into(),
        description: "Collaborative spreadsheets of the Facile suite. Paths under `/api` need a session: the \
            `session` cookie (send `X-Facile-CSRF` on writes) or `Authorization: Bearer`. Errors are \
            `{\"error\":{\"code\",\"message\"}}`."
            .into(),
        ..Config::default()
    }
}

/// Every route, as OpenAPI 3.1, with the MCP routes when they are mounted.
pub fn document(mcp: bool) -> OpenApi {
    let mcp = if mcp { auth::MCP } else { &[] };
    let routes = auth::ROUTES.iter().chain(workbooks::ROUTES).chain(mcp);
    let paths = routes.fold(PathsBuilder::new(), |paths, route| paths.path(route.path, route.item()));
    apiref::document(&config(), OpenApiBuilder::new().paths(paths.build()).build())
}

/// `GET /docs` and `GET /docs/openapi.json`.
pub fn routes(mcp: bool) -> Result<Router, String> {
    apiref::routes(&config(), document(mcp))
}
