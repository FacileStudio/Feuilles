mod common;

use common::web::{call, create, import};
use common::{Person, TestApp, app, person, space};
use http::{Method, StatusCode};
use serde_json::{Value, json};

struct Cast {
    owner: Person,
    admin: Person,
    member: Person,
    outsider: Person,
    space: String,
}

impl Cast {
    fn everyone(&self) -> [(&'static str, &Person); 4] {
        [("owner", &self.owner), ("admin", &self.admin), ("member", &self.member), ("outsider", &self.outsider)]
    }
}

async fn cast(app: &TestApp) -> Cast {
    let (owner, admin, member, outsider) =
        (person(&app.state).await, person(&app.state).await, person(&app.state).await, person(&app.state).await);
    let shared = space(&app.state, &[(&owner, "owner"), (&admin, "admin"), (&member, "member")]).await;
    space(&app.state, &[(&outsider, "owner")]).await;
    Cast { owner, admin, member, outsider, space: shared }
}

struct Route {
    method: Method,
    suffix: &'static str,
    body: Option<Value>,
    trashed: bool,
    ok: StatusCode,
}

fn routes() -> Vec<(Route, [bool; 3])> {
    let route = |method, suffix, body, ok| Route { method, suffix, body, trashed: false, ok };
    let write = json!({"writes": [{"sheet": 0, "row": 1, "column": 1, "input": "1"}]});
    let mut permanent = route(Method::DELETE, "/permanent", None, StatusCode::NO_CONTENT);
    permanent.trashed = true;
    vec![
        (route(Method::GET, "", None, StatusCode::OK), [true, true, true]),
        (route(Method::PATCH, "", Some(json!({"title": "New"})), StatusCode::OK), [true, true, true]),
        (route(Method::DELETE, "", None, StatusCode::NO_CONTENT), [true, true, false]),
        (route(Method::POST, "/restore", None, StatusCode::OK), [true, true, false]),
        (permanent, [true, true, false]),
        (route(Method::GET, "/export.xlsx", None, StatusCode::OK), [true, true, true]),
        (route(Method::GET, "/export.csv", None, StatusCode::OK), [true, true, true]),
        (route(Method::GET, "/cells?range=A1", None, StatusCode::OK), [true, true, true]),
        (route(Method::PUT, "/cells", Some(write), StatusCode::OK), [true, true, true]),
    ]
}

async fn fresh(app: &TestApp, author: &Person, space: Option<&str>, trashed: bool) -> String {
    let id = create(&app.router, author, space).await["id"].as_str().expect("id").to_owned();
    if trashed {
        let trash = call(&app.router, Method::DELETE, &format!("/api/workbooks/{id}"), Some(&author.token), None).await;
        assert_eq!(trash.0, StatusCode::NO_CONTENT);
    }
    id
}

async fn status(app: &TestApp, route: &Route, id: &str, who: Option<&Person>) -> StatusCode {
    let path = format!("/api/workbooks/{id}{}", route.suffix);
    call(&app.router, route.method.clone(), &path, who.map(|p| p.token.as_str()), route.body.clone()).await.0
}

#[tokio::test]
async fn every_workbook_route_answers_each_role_of_a_space() {
    let Some(app) = app().await else { return };
    let cast = cast(&app).await;
    for (route, allowed) in routes() {
        for ((role, who), expected) in cast.everyone().into_iter().zip(allowed.iter().copied().chain([false])) {
            let id = fresh(&app, &cast.owner, Some(&cast.space), route.trashed).await;
            let want = match (expected, role) {
                (true, _) => route.ok,
                (false, "outsider") => StatusCode::NOT_FOUND,
                (false, _) => StatusCode::FORBIDDEN,
            };
            assert_eq!(status(&app, &route, &id, Some(who)).await, want, "{} {} as {role}", route.method, route.suffix);
        }
        let id = fresh(&app, &cast.owner, Some(&cast.space), route.trashed).await;
        assert_eq!(
            status(&app, &route, &id, None).await,
            StatusCode::UNAUTHORIZED,
            "{} {} anonymous",
            route.method,
            route.suffix
        );
    }
}

#[tokio::test]
async fn a_personal_workbook_is_its_owner_alone_and_admins_get_no_bypass() {
    let Some(app) = app().await else { return };
    let cast = cast(&app).await;
    for (route, _) in routes() {
        let id = fresh(&app, &cast.member, None, route.trashed).await;
        assert_eq!(status(&app, &route, &id, Some(&cast.member)).await, route.ok, "{} {}", route.method, route.suffix);
        for (role, who) in [("owner", &cast.owner), ("admin", &cast.admin), ("outsider", &cast.outsider)] {
            let id = fresh(&app, &cast.member, None, route.trashed).await;
            let got = status(&app, &route, &id, Some(who)).await;
            assert_eq!(got, StatusCode::NOT_FOUND, "{} {} as space {role}", route.method, route.suffix);
        }
    }
}

#[tokio::test]
async fn list_create_and_import_scope_to_spaces_the_caller_is_in() {
    let Some(app) = app().await else { return };
    let cast = cast(&app).await;
    let list = format!("/api/workbooks?space_id={}", cast.space);
    let body = json!({"title": "In the space", "space_id": cast.space});
    let xlsx =
        api::engine::export_xlsx(&api::engine::new_model("Book", "en", "UTC", "en").expect("model")).expect("xlsx");
    for (role, who) in cast.everyone() {
        let want_list = if role == "outsider" { StatusCode::NOT_FOUND } else { StatusCode::OK };
        let want_create = if role == "outsider" { StatusCode::NOT_FOUND } else { StatusCode::CREATED };
        assert_eq!(call(&app.router, Method::GET, &list, Some(&who.token), None).await.0, want_list, "list as {role}");
        let created = call(&app.router, Method::POST, "/api/workbooks", Some(&who.token), Some(body.clone())).await;
        assert_eq!(created.0, want_create, "create as {role}");
        assert_eq!(
            import(&app.router, &who.token, "book.xlsx", &xlsx, Some(&cast.space)).await.0,
            want_create,
            "import as {role}"
        );
        assert_eq!(call(&app.router, Method::GET, "/api/workbooks", Some(&who.token), None).await.0, StatusCode::OK);
    }
    assert_eq!(call(&app.router, Method::GET, &list, None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app.router, Method::POST, "/api/workbooks", None, Some(body)).await.0, StatusCode::UNAUTHORIZED);
}
