mod common;

use axum::Router;
use axum::body::Body;
use common::agent::{args, options, token};
use common::web::{browser_login, call, send};
use common::{Person, person, serve_with, state};
use http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tronc::mcptest;

fn from_browser(method: Method, path: &str, cookie: &str) -> Request<Body> {
    let cookie = format!("{}={cookie}", porte::SESSION_COOKIE_NAME);
    let builder = Request::builder().method(method).uri(path).header(header::COOKIE, cookie);
    builder.header(porte::CSRF_HEADER_NAME, "1").body(Body::empty()).expect("request")
}

async fn page(router: &Router, method: Method, path: &str, cookie: &str) -> (StatusCode, Value) {
    let (status, body) = send(router, from_browser(method, path, cookie)).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn delete_request(client: &mcptest::Client) -> (String, String) {
    let created = mcptest::call(client, "feuilles_create_workbook", args(json!({"title": "Plan"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let write = json!({"workbook_id": id, "writes": [{"sheet": 0, "row": 2, "column": 1, "input": "keep"}]});
    mcptest::call(client, "feuilles_write_cells", args(write)).await;
    let action = ask_delete(client, &id).await;
    (id, action)
}

async fn ask_delete(client: &mcptest::Client, id: &str) -> String {
    let ask = json!({"workbook_id": id, "sheet": "Sheet1", "at": 1, "count": 1});
    let pending = mcptest::decode::<Value>(&mcptest::call(client, "feuilles_delete_rows", args(ask)).await);
    assert_eq!(pending["status"], "pending_confirmation");
    assert!(pending["message"].as_str().expect("message").starts_with("Nothing happened yet"), "{pending}");
    pending["confirm_url"].as_str().expect("link").rsplit('=').next().expect("id").to_owned()
}

async fn plain_cookie(state: &api::state::AppState, who: &Person, token: &str) -> String {
    let store = porte::pg::Store::new(state.pool.clone()).sessions();
    let expires_at = Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    let row =
        porte::Session { user_id: who.user_id, token_hash: porte::hash_token(token), expires_at, ..Default::default() };
    store.create(row).await.expect("session row");
    token.to_owned()
}

async fn assert_unframable(router: &Router, cookie: &str) {
    let response = tower::ServiceExt::oneshot(router.clone(), from_browser(Method::GET, "/api/mcp/pending", cookie));
    let response = response.await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let policy = response.headers()["content-security-policy"].to_str().expect("policy");
    assert!(policy.contains("frame-ancestors 'none'"), "{policy}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delete_waits_for_the_person_and_only_their_browser_login_confirms_it() {
    let Some(state) = state().await else { return };
    let (me, other) = (person(&state).await, person(&state).await);
    let (mine, theirs) = (browser_login(&state, &me).await, browser_login(&state, &other).await);
    assert!(mine.starts_with(porte::BROWSER_TOKEN_PREFIX));
    let addr = serve_with(state.clone(), options).await;
    let router = api::router::build_router(state.clone(), options(addr)).expect("router");
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let (id, action) = delete_request(&client).await;
    assert_eq!(ask_delete(&client, &id).await, action, "asking twice reuses the open action");
    let (status, listed) = page(&router, Method::GET, "/api/mcp/pending", &mine).await;
    assert_eq!(status, StatusCode::OK);
    let first = &listed["actions"][0];
    assert_eq!(listed["actions"].as_array().map(Vec::len), Some(1));
    assert_eq!(first["id"].as_str(), Some(action.as_str()));
    assert_eq!(first["summary"], "Delete row 1 of 'Sheet1' in \"Plan\" with their cells?");
    assert_eq!(first["state"], "ready");
    assert_eq!(
        serde_json::from_str::<Value>(first["target"].as_str().expect("target")).expect("json")["workbook_id"],
        id
    );
    let confirm = format!("/api/mcp/pending/{action}/confirm");
    let bearer = call(&router, Method::POST, &confirm, Some(&me.token), None).await;
    assert_eq!(bearer.0, StatusCode::FORBIDDEN, "{}", bearer.1);
    assert_eq!(page(&router, Method::POST, &confirm, &theirs).await.0, StatusCode::NOT_FOUND);
    assert_eq!(page(&router, Method::POST, &confirm, &mine).await, (StatusCode::OK, json!({"status": "done"})));
    let read = mcptest::call(&client, "feuilles_read_range", args(json!({"workbook_id": id, "range": "A1"}))).await;
    assert_eq!(mcptest::decode::<Value>(&read)["rows"][0][0]["input"], "keep");
    assert_eq!(page(&router, Method::POST, &confirm, &mine).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cli_api_or_old_token_sent_as_the_cookie_cannot_confirm() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let addr = serve_with(state.clone(), options).await;
    let router = api::router::build_router(state.clone(), options(addr)).expect("router");
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let (_, action) = delete_request(&client).await;
    let (cli, _) = state.kits.sessions.issue(me.user_id, "").await.expect("cli session");
    let older = plain_cookie(&state, &me, &format!("b-{}", porte::new_token().expect("token"))).await;
    let unmarked = plain_cookie(&state, &me, &porte::new_token().expect("token")).await;
    let denied = json!({"error": {"code": "permission_denied", "message": tronc::mcp::DELEGATED}});
    for (name, cookie) in
        [("named api token", &me.token), ("cli exchange", &cli), ("b- cookie", &older), ("unmarked", &unmarked)]
    {
        assert_eq!(page(&router, Method::GET, "/api/mcp/pending", cookie).await.0, StatusCode::FORBIDDEN, "{name}");
        let confirm = format!("/api/mcp/pending/{action}/confirm");
        assert_eq!(
            page(&router, Method::POST, &confirm, cookie).await,
            (StatusCode::FORBIDDEN, denied.clone()),
            "{name}"
        );
    }
    let mine = browser_login(&state, &me).await;
    assert_unframable(&router, &mine).await;
    let dismiss = format!("/api/mcp/pending/{action}/dismiss");
    assert_eq!(page(&router, Method::POST, &dismiss, &mine).await, (StatusCode::OK, json!({"status": "dismissed"})));
}
