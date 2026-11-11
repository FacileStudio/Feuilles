mod common;

use axum::body::Body;
use common::web::{call, create, import, send};
use common::{app, person};
use http::{Method, Request, StatusCode, header};
use serde_json::json;

#[tokio::test]
async fn create_list_rename_trash_restore_and_delete() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let first = create(&app.router, &me, None).await;
    let second = create(&app.router, &me, None).await;
    assert_eq!(first["sheets"][0]["name"], "Sheet1");
    assert_eq!(first["engine_version"], api::engine::ENGINE_VERSION);
    let id = first["id"].as_str().expect("id").to_owned();
    let path = format!("/api/workbooks/{id}");
    let (status, renamed) =
        call(&app.router, Method::PATCH, &path, Some(&me.token), Some(json!({"title": "  Q3  "}))).await;
    assert_eq!((status, renamed["title"].as_str()), (StatusCode::OK, Some("Q3")));
    let (_, list) = call(&app.router, Method::GET, "/api/workbooks", Some(&me.token), None).await;
    let ids: Vec<&str> = list["workbooks"].as_array().expect("list").iter().filter_map(|w| w["id"].as_str()).collect();
    assert_eq!(ids, [id.as_str(), second["id"].as_str().expect("id")]);
    let (status, _) = call(&app.router, Method::DELETE, &format!("{path}/permanent"), Some(&me.token), None).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(call(&app.router, Method::DELETE, &path, Some(&me.token), None).await.0, StatusCode::NO_CONTENT);
    let (_, trash) = call(&app.router, Method::GET, "/api/workbooks?trashed=true", Some(&me.token), None).await;
    assert_eq!(trash["workbooks"][0]["id"], id.as_str());
    assert!(trash["workbooks"][0]["trashed_at"].is_string());
    let (status, restored) = call(&app.router, Method::POST, &format!("{path}/restore"), Some(&me.token), None).await;
    assert_eq!((status, restored["trashed_at"].is_null()), (StatusCode::OK, true));
    call(&app.router, Method::DELETE, &path, Some(&me.token), None).await;
    let (status, _) = call(&app.router, Method::DELETE, &format!("{path}/permanent"), Some(&me.token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(call(&app.router, Method::GET, &path, Some(&me.token), None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn bad_bodies_are_refused() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let token = Some(me.token.as_str());
    for body in [json!({"title": "   "}), json!({"title": "x".repeat(201)}), json!({"title": "a", "colour": 1})] {
        let (status, error) = call(&app.router, Method::POST, "/api/workbooks", token, Some(body)).await;
        assert_eq!((status, error["error"]["code"].as_str()), (StatusCode::BAD_REQUEST, Some("invalid_argument")));
    }
    let (status, _) =
        call(&app.router, Method::POST, "/api/workbooks", token, Some(json!({"title": "a", "language": "xx"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app.router, Method::GET, "/api/workbooks/not-a-uuid", token, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app.router, Method::GET, "/api/nothing-here", token, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cells_are_written_as_one_batch_and_read_back() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let id = create(&app.router, &me, None).await["id"].as_str().expect("id").to_owned();
    let cells = format!("/api/workbooks/{id}/cells");
    let writes = json!({"writes": [
        {"sheet": 0, "row": 1, "column": 1, "input": "20"},
        {"sheet": 0, "row": 2, "column": 1, "input": "=A1*2+2"}
    ]});
    let (status, answer) = call(&app.router, Method::PUT, &cells, Some(&me.token), Some(writes)).await;
    assert_eq!((status, answer["seq"].as_u64()), (StatusCode::OK, Some(1)));
    let (status, read) = call(&app.router, Method::GET, &format!("{cells}?range=A2:A1"), Some(&me.token), None).await;
    assert_eq!((status, read["range"].as_str()), (StatusCode::OK, Some("A1:A2")));
    assert_eq!(read["rows"][1][0], json!({"input": "=A1*2+2", "value": 42.0, "formatted": "42"}));
    let too_big = format!("{cells}?range=A1:Z1000");
    assert_eq!(call(&app.router, Method::GET, &too_big, Some(&me.token), None).await.0, StatusCode::BAD_REQUEST);
    let missing_sheet = json!({"writes": [{"sheet": 4, "row": 1, "column": 1, "input": "x"}]});
    assert_eq!(
        call(&app.router, Method::PUT, &cells, Some(&me.token), Some(missing_sheet)).await.0,
        StatusCode::NOT_FOUND
    );
    let empty = json!({"writes": []});
    assert_eq!(call(&app.router, Method::PUT, &cells, Some(&me.token), Some(empty)).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn xlsx_and_csv_go_out_and_xlsx_comes_back_in() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let id = create(&app.router, &me, None).await["id"].as_str().expect("id").to_owned();
    let writes = json!({"writes": [{"sheet": 0, "row": 1, "column": 1, "input": "7"}, {"sheet": 0, "row": 1, "column": 2, "input": "=A1*6"}]});
    call(&app.router, Method::PUT, &format!("/api/workbooks/{id}/cells"), Some(&me.token), Some(writes)).await;
    let (status, csv) = send(
        &app.router,
        common::web::request(Method::GET, &format!("/api/workbooks/{id}/export.csv"), Some(&me.token), None),
    )
    .await;
    assert_eq!((status, String::from_utf8_lossy(&csv).as_ref()), (StatusCode::OK, "7,42\r\n"));
    let export = common::web::request(Method::GET, &format!("/api/workbooks/{id}/export.xlsx"), Some(&me.token), None);
    let response = tower::ServiceExt::oneshot(app.router.clone(), export).await.expect("response");
    let disposition = response.headers()[header::CONTENT_DISPOSITION].to_str().expect("header").to_owned();
    assert!(disposition.starts_with("attachment; filename=\"Budget.xlsx\""), "{disposition}");
    let xlsx = http_body_util::BodyExt::collect(response.into_body()).await.expect("body").to_bytes();
    let (status, imported) = import(&app.router, &me.token, "Q4 plan.xlsx", &xlsx, None).await;
    assert_eq!((status, imported["title"].as_str()), (StatusCode::CREATED, Some("Q4 plan")));
    let cells = format!("/api/workbooks/{}/cells?range=B1", imported["id"].as_str().expect("id"));
    let (_, read) = call(&app.router, Method::GET, &cells, Some(&me.token), None).await;
    assert_eq!(read["rows"][0][0]["formatted"], "42");
    assert_eq!(import(&app.router, &me.token, "notes.txt", b"hello", None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(import(&app.router, &me.token, "broken.xlsx", b"not a zip", None).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_csv_upload_becomes_a_one_sheet_workbook_named_after_the_file() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let file = "\u{feff}Nom;Montant\r\nAda;1,5\r\nBob;=A2\r\n";
    let (status, imported) = import(&app.router, &me.token, "Clients.CSV", file.as_bytes(), None).await;
    assert_eq!((status, imported["title"].as_str()), (StatusCode::CREATED, Some("Clients")));
    assert_eq!(imported["sheets"], json!([{"index": 0, "name": "Clients", "used_range": "A1:B3"}]));
    let cells = format!("/api/workbooks/{}/cells?range=B2:B3", imported["id"].as_str().expect("id"));
    let (_, read) = call(&app.router, Method::GET, &cells, Some(&me.token), None).await;
    assert_eq!(
        (read["rows"][0][0]["formatted"].as_str(), read["rows"][1][0]["formatted"].as_str()),
        (Some("1.5"), Some("=A2"))
    );
    assert_eq!(import(&app.router, &me.token, "empty.csv", b"", None).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_cookie_mutation_needs_the_csrf_header() {
    let Some(app) = app().await else { return };
    let me = person(&app.state).await;
    let post = |csrf: bool| {
        let mut builder = Request::builder()
            .method(Method::POST)
            .uri("/api/workbooks")
            .header(header::COOKIE, format!("session={}", me.token))
            .header(header::CONTENT_TYPE, "application/json");
        if csrf {
            builder = builder.header("X-Facile-CSRF", "1");
        }
        builder.body(Body::from(json!({"title": "t"}).to_string())).expect("request")
    };
    assert_eq!(send(&app.router, post(false)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send(&app.router, post(true)).await.0, StatusCode::CREATED);
}
