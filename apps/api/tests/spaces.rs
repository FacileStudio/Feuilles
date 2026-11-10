mod common;

use common::web::call;
use common::{app, person, space};
use http::{Method, StatusCode};
use serde_json::json;

#[tokio::test]
async fn spaces_lists_only_the_callers_own() {
    let Some(app) = app().await else { return };
    let (admin, outsider, member) = (person(&app.state).await, person(&app.state).await, person(&app.state).await);
    let shared = space(&app.state, &[(&admin, "admin"), (&member, "member")]).await;
    space(&app.state, &[(&outsider, "owner")]).await;
    let (status, answer) = call(&app.router, Method::GET, "/api/spaces", Some(&admin.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["spaces"], json!([{"id": shared, "name": shared, "role": "admin"}]));
    let (_, answer) = call(&app.router, Method::GET, "/api/spaces", Some(&outsider.token), None).await;
    assert_eq!(answer["spaces"].as_array().map(Vec::len), Some(1));
    assert_ne!(answer["spaces"][0]["id"], shared.as_str());
    assert_eq!(call(&app.router, Method::GET, "/api/spaces", None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, me) = call(&app.router, Method::GET, "/api/auth/me", Some(&member.token), None).await;
    assert_eq!((status, me["id"].as_i64()), (StatusCode::OK, Some(member.user_id)));
    assert_eq!(call(&app.router, Method::GET, "/api/auth/me", None, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, session) = call(&app.router, Method::GET, "/api/auth/session", None, None).await;
    assert_eq!((status, session), (StatusCode::OK, json!({"user": null})));
}
