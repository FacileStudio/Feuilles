mod common;

use common::web::send;
use http::{Method, StatusCode};

/// Routes the libraries mount with `any()` and answer 405 to the wrong methods themselves, so
/// the router lists them as `ANY`: porte's deletions receiver and tronc's MCP routes. The
/// document has each under the methods it serves.
const ANY_METHOD: [&str; 3] = ["/api/registre/deletions", "/mcp", "/.well-known/oauth-protected-resource"];

struct Never;

impl tronc::mcp::Verifier for Never {
    fn verify<'a>(&'a self, _: &'a str, _: &'a http::request::Parts) -> tronc::mcp::VerifyFuture<'a> {
        Box::pin(async { Err(tronc::mcp::VerifyError::Invalid) })
    }
}

#[tokio::test]
async fn every_mounted_route_is_documented() {
    let Some(state) = common::state().await else { return };
    for mcp in [false, true] {
        let options = api::router::Options {
            mcp: mcp.then(|| tronc::mcp::AppConfig {
                state_key: api::mcp::state_key("test-key"),
                ..tronc::mcp::AppConfig::new("http://127.0.0.1:4000/mcp", common::ISSUER, std::sync::Arc::new(Never))
            }),
            ..Default::default()
        };
        let router = api::router::build_router(state.clone(), options).expect("router");
        let missing = tronc::apiref::undocumented(
            &router,
            &api::reference::document(mcp),
            &api::reference::config(),
            &ANY_METHOD,
        );
        assert!(missing.is_empty(), "routes missing from the API document (mcp {mcp}): {missing:?}");
    }
}

#[tokio::test]
async fn the_reference_page_and_its_document_are_served() {
    let Some(app) = common::app().await else { return };
    let (status, page) = send(&app.router, common::web::request(Method::GET, "/docs", None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&page).contains("/docs/openapi.json"));
    let (status, spec) = send(&app.router, common::web::request(Method::GET, "/docs/openapi.json", None, None)).await;
    let spec: serde_json::Value = serde_json::from_slice(&spec).expect("json");
    assert_eq!(status, StatusCode::OK);
    assert!(spec["paths"]["/api/workbooks/{id}/ws"]["get"].is_object());
}
