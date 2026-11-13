mod common;

use axum::Router;
use axum::body::Body;
use common::agent::{args, options, token};
use common::web::{call, send};
use common::{Person, person, serve_with, state};
use http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tronc::mcptest;

fn from_browser(method: Method, path: &str, who: &Person) -> Request<Body> {
    let cookie = format!("{}={}", porte::SESSION_COOKIE_NAME, who.token);
    let builder = Request::builder().method(method).uri(path).header(header::COOKIE, cookie);
    builder.header(porte::CSRF_HEADER_NAME, "1").body(Body::empty()).expect("request")
}

async fn page(router: &Router, method: Method, path: &str, who: &Person) -> (StatusCode, Value) {
    let (status, body) = send(router, from_browser(method, path, who)).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delete_waits_for_the_person_and_only_their_browser_confirms_it() {
    let Some(state) = state().await else { return };
    let (me, other) = (person(&state).await, person(&state).await);
    let addr = serve_with(state.clone(), options).await;
    let router = api::router::build_router(state.clone(), options(addr)).expect("router");
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Plan"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let write = json!({"workbook_id": id, "writes": [{"sheet": 0, "row": 2, "column": 1, "input": "keep"}]});
    mcptest::call(&client, "feuilles_write_cells", args(write)).await;
    let ask = json!({"workbook_id": id, "sheet": "Sheet1", "at": 1, "count": 1});
    let pending = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_delete_rows", args(ask)).await);
    assert_eq!(pending["status"], "pending_confirmation");
    let action = pending["confirm_url"].as_str().expect("link").rsplit('=').next().expect("id").to_owned();
    let (status, listed) = page(&router, Method::GET, "/api/mcp/actions", &me).await;
    assert_eq!((status, listed[0]["id"].as_str()), (StatusCode::OK, Some(action.as_str())));
    assert_eq!(listed[0]["summary"], "Delete row 1 of 'Sheet1' in \"Plan\" with their cells?");
    assert_eq!(listed[0]["state"], "ready");
    let confirm = format!("/api/mcp/actions/{action}/confirm");
    let bearer = call(&router, Method::POST, &confirm, Some(&me.token), None).await;
    assert_eq!(bearer.0, StatusCode::FORBIDDEN, "{}", bearer.1);
    assert_eq!(page(&router, Method::POST, &confirm, &other).await.0, StatusCode::NOT_FOUND);
    assert_eq!(page(&router, Method::POST, &confirm, &me).await, (StatusCode::OK, json!({"status": "done"})));
    let read = mcptest::call(&client, "feuilles_read_range", args(json!({"workbook_id": id, "range": "A1"}))).await;
    assert_eq!(mcptest::decode::<Value>(&read)["rows"][0][0]["input"], "keep");
    assert_eq!(page(&router, Method::POST, &confirm, &me).await.0, StatusCode::NOT_FOUND);
}
