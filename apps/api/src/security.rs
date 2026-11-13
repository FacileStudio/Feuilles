use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use tronc::http::{SecurityConfig, SecurityOverride, SecurityOverrideConfig, Sources, security_override};

/// tronc's policy plus what the Feuilles client needs: `wasm-unsafe-eval` to compile the
/// IronCalc engine, and the hashes of the inline scripts adapter-static writes. The policy stays
/// report-only, like every suite app's.
pub fn config(client_dir: &Path) -> SecurityConfig {
    let mut scripts = vec!["'wasm-unsafe-eval'".to_owned()];
    if tronc::spa::available(client_dir) {
        match tronc::spa::inline_script_hashes(client_dir) {
            Ok(hashes) => scripts.extend(hashes),
            Err(error) => tracing::error!(%error, "failed to hash the client's inline scripts"),
        }
    }
    SecurityConfig { csp_sources: Sources::from([("script-src".to_owned(), scripts)]), ..SecurityConfig::default() }
}

/// Who may frame the client: Feuilles itself and Docs, which embeds workbooks with `?embed=1`,
/// plus any localhost port outside production.
pub fn frame_ancestors(production: bool) -> Vec<String> {
    let mut sources = vec!["'self'".to_owned(), "https://docs.facile.studio".to_owned()];
    if !production {
        sources.push("http://localhost:*".to_owned());
    }
    sources
}

/// The built client, answering every path the other routes leave, framable by
/// [`frame_ancestors`]. The API keeps tronc's `frame-ancestors 'none'`.
pub fn spa(dir: PathBuf, production: bool) -> Result<Router, String> {
    let framing = SecurityOverride::new(SecurityOverrideConfig {
        csp_sources: Sources::from([("frame-ancestors".to_owned(), frame_ancestors(production))]),
        ..SecurityOverrideConfig::default()
    })?;
    let handler = tronc::spa::handler(tronc::spa::Config { dir, ..Default::default() });
    Ok(Router::new()
        .fallback_service(handler)
        .layer(axum::middleware::from_fn_with_state(Arc::new(framing), security_override)))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::get;
    use http::Request;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn the_client_can_be_framed_by_docs_and_the_api_cannot() {
        let dir = std::env::temp_dir().join(format!("feuilles-spa-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("index.html"), "<!doctype html><title>Feuilles</title>").expect("index");
        let http = tronc::http::Config { security: config(&dir), ..Default::default() };
        let app = Router::new()
            .route("/api/ping", get(|| async { "pong" }))
            .fallback_service(spa(dir.clone(), true).expect("spa"))
            .layer(tronc::http::layers(http).expect("layers"));
        let headers = |path: &'static str| {
            let app = app.clone();
            async move { app.oneshot(Request::get(path).body(Body::empty()).expect("request")).await.expect("response") }
        };
        let page = headers("/w/123?embed=1").await;
        let policy = page.headers()["content-security-policy"].to_str().expect("policy").to_owned();
        assert!(policy.contains("frame-ancestors 'self' https://docs.facile.studio"), "{policy}");
        assert!(!policy.contains("localhost"), "{policy}");
        assert!(page.headers().get("x-frame-options").is_none());
        let api = headers("/api/ping").await;
        assert!(api.headers()["content-security-policy"].to_str().expect("policy").contains("frame-ancestors 'none'"));
        assert_eq!(api.headers()["x-frame-options"], "DENY");
        std::fs::remove_dir_all(dir).expect("cleanup");
    }
}
