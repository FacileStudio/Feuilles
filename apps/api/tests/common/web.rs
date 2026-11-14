use axum::Router;
use axum::body::Body;
use http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use super::Person;

pub fn request(method: Method, path: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    builder.body(body).expect("request")
}

pub async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    (status, response.into_body().collect().await.expect("body").to_bytes().to_vec())
}

pub async fn call(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let (status, bytes) = send(router, request(method, path, token, body)).await;
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

pub async fn create(router: &Router, person: &Person, space: Option<&str>) -> Value {
    let body = serde_json::json!({ "title": "Budget", "space_id": space });
    let (status, workbook) = call(router, Method::POST, "/api/workbooks", Some(&person.token), Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{workbook}");
    workbook
}

pub fn multipart(fields: &[(&str, Option<&str>, &[u8])]) -> (String, Vec<u8>) {
    let boundary = "feuillesboundary".to_owned();
    let mut body = Vec::new();
    for (name, file, bytes) in fields {
        body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"").as_bytes());
        if let Some(file) = file {
            body.extend(format!("; filename=\"{file}\"").as_bytes());
        }
        body.extend(b"\r\n\r\n");
        body.extend(*bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

pub async fn import(
    router: &Router,
    token: &str,
    name: &str,
    bytes: &[u8],
    space: Option<&str>,
) -> (StatusCode, Value) {
    let mut fields: Vec<(&str, Option<&str>, &[u8])> = vec![("file", Some(name), bytes)];
    if let Some(space) = space {
        fields.push(("space_id", None, space.as_bytes()));
    }
    let (content_type, body) = multipart(&fields);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/workbooks/import")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .expect("request");
    let (status, bytes) = send(router, request).await;
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// A browser login's session token for `who`, as porte's OIDC callback mints it.
pub async fn browser_login(state: &api::state::AppState, who: &Person) -> String {
    let template = porte::Session { user_id: who.user_id, ..porte::Session::default() };
    let request = http::Request::new(());
    state.kits.sessions.issue_cookie_for(&request, template).await.expect("browser session").0
}
