use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use tronc::http::{SecurityConfig, SecurityOverride, SecurityOverrideConfig, Sources, security_override};
use tronc::mcp::PENDING_PATH;

/// The origin the landing page's plan cards read the suite catalog from.
const PLAN_CATALOG: &str = "https://api-ardoise.facile.studio";

/// tronc's policy plus what the Feuilles client needs: `wasm-unsafe-eval` to compile the
/// IronCalc engine, Vision's tracker script, the hashes of the inline scripts adapter-static
/// writes, `frame-src` for the Bulle sidebar at `bulle`, and `connect-src` for the plan catalog.
/// The policy stays report-only, like every suite app's.
pub fn config(client_dir: &Path, bulle: &str) -> SecurityConfig {
    let mut scripts = vec!["'wasm-unsafe-eval'".to_owned(), "https://vision.facile.studio".to_owned()];
    if tronc::spa::available(client_dir) {
        match tronc::spa::inline_script_hashes(client_dir) {
            Ok(hashes) => scripts.extend(hashes),
            Err(error) => tracing::error!(%error, "failed to hash the client's inline scripts"),
        }
    }
    let sources = Sources::from([
        ("script-src".to_owned(), scripts),
        ("frame-src".to_owned(), vec![bulle.to_owned()]),
        ("connect-src".to_owned(), vec![PLAN_CATALOG.to_owned()]),
    ]);
    SecurityConfig { csp_sources: sources, ..SecurityConfig::default() }
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

/// The Bulle origin in `raw`, `BULLE_URL`, without a trailing slash: an http(s) origin with no
/// path, or [`crate::env::DEFAULT_BULLE`] when `raw` is blank.
pub fn bulle_origin(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok(crate::env::DEFAULT_BULLE.to_owned());
    }
    let refused = || format!("BULLE_URL {raw:?} must be an http(s) origin, without a path");
    let uri: http::Uri = trimmed.parse().map_err(|_| refused())?;
    let bare = matches!(uri.scheme_str(), Some("https" | "http"))
        && uri.host().is_some_and(|host| !host.is_empty())
        && uri.path_and_query().is_none_or(|path| matches!(path.as_str(), "" | "/"));
    bare.then(|| trimmed.to_owned()).ok_or_else(refused)
}

/// The built client, answering every path the other routes leave, framable by
/// [`frame_ancestors`] except the confirm page, which keeps tronc's `frame-ancestors 'none'` like
/// the API: an approval inside a Docs embed would not be the person's own.
pub fn spa(dir: PathBuf, production: bool) -> Result<Router, String> {
    let framing = SecurityOverride::new(SecurityOverrideConfig {
        csp_sources: Sources::from([("frame-ancestors".to_owned(), frame_ancestors(production))]),
        ..SecurityOverrideConfig::default()
    })?;
    let handler = tronc::spa::handler(tronc::spa::Config { dir, ..Default::default() });
    Ok(Router::new().fallback_service(handler).layer(axum::middleware::from_fn_with_state(Arc::new(framing), framed)))
}

async fn framed(policy: State<Arc<SecurityOverride>>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if path == PENDING_PATH || path.starts_with(&format!("{PENDING_PATH}/")) {
        return next.run(request).await;
    }
    security_override(policy, request, next).await
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::get;
    use http::Request;
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn bulle_is_an_origin_with_the_suite_default() {
        assert_eq!(bulle_origin(" ").as_deref(), Ok(crate::env::DEFAULT_BULLE));
        assert_eq!(bulle_origin("http://localhost:5174/").as_deref(), Ok("http://localhost:5174"));
        for bad in ["javascript:alert(1)", "https://bulle.test/embed", "bulle.test", "https://bulle.test?x=1"] {
            assert!(bulle_origin(bad).is_err(), "{bad}");
        }
    }

    #[tokio::test]
    async fn the_client_can_be_framed_by_docs_and_frame_bulle_and_the_api_cannot() {
        let dir = std::env::temp_dir().join(format!("feuilles-spa-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("index.html"), "<!doctype html><title>Feuilles</title>").expect("index");
        let http = tronc::http::Config { security: config(&dir, "https://bulle.test"), ..Default::default() };
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
        let report = page.headers()["content-security-policy-report-only"].to_str().expect("report-only").to_owned();
        assert!(report.contains("frame-src https://bulle.test"), "{report}");
        assert!(report.contains("connect-src 'self' https://api-ardoise.facile.studio"), "{report}");
        assert!(page.headers().get("x-frame-options").is_none());
        for confirm in ["/pending?action=a1", "/pending/"] {
            let page = headers(confirm).await;
            let policy = page.headers()["content-security-policy"].to_str().expect("policy").to_owned();
            assert!(policy.contains("frame-ancestors 'none'"), "{confirm}: {policy}");
            assert_eq!(page.headers()["x-frame-options"], "DENY", "{confirm}");
        }
        let api = headers("/api/ping").await;
        assert!(api.headers()["content-security-policy"].to_str().expect("policy").contains("frame-ancestors 'none'"));
        assert_eq!(api.headers()["x-frame-options"], "DENY");
        std::fs::remove_dir_all(dir).expect("cleanup");
    }
}
