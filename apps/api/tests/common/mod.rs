#![allow(dead_code)]

use api::hub::Limits;
use api::router::{self, Options};
use api::state::AppState;
use std::net::SocketAddr;

use axum::Router;
use uuid::Uuid;

pub mod agent;
pub mod directory;
mod frames;
pub mod sheet;
mod state;
pub mod web;
pub mod ws;

pub use state::state_in;

pub const ISSUER: &str = "https://sso.test";

pub struct TestApp {
    pub state: AppState,
    pub router: Router,
}

pub struct Person {
    pub user_id: i64,
    pub subject: String,
    pub token: String,
}

pub async fn state() -> Option<AppState> {
    state_with(Limits::default()).await
}

pub async fn state_with(limits: Limits) -> Option<AppState> {
    state_in("feuilles", limits).await
}

pub async fn app() -> Option<TestApp> {
    let state = state().await?;
    let router = router::build_router(state.clone(), Options::default()).expect("router");
    Some(TestApp { state, router })
}

pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4().simple())
}

pub async fn person(state: &AppState) -> Person {
    let subject = unique("sub");
    let user_id: i64 = sqlx::query_scalar("INSERT INTO porte_users (email, name) VALUES ($1, $2) RETURNING id")
        .bind(format!("{subject}@example.test"))
        .bind(&subject)
        .fetch_one(&state.pool)
        .await
        .expect("user");
    sqlx::query("INSERT INTO porte_identities (user_id, provider, subject) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(ISSUER)
        .bind(&subject)
        .execute(&state.pool)
        .await
        .expect("identity");
    let (token, _) = state.kits.sessions.issue(user_id, "test").await.expect("session");
    Person { user_id, subject, token }
}

pub async fn space(state: &AppState, members: &[(&Person, &str)]) -> String {
    let id = unique("space");
    sqlx::query("INSERT INTO spaces (id, name) VALUES ($1, $1)").bind(&id).execute(&state.pool).await.expect("space");
    for (person, role) in members {
        sqlx::query("INSERT INTO space_members (space_id, subject, email, role) VALUES ($1, $2, '', $3)")
            .bind(&id)
            .bind(&person.subject)
            .bind(role)
            .execute(&state.pool)
            .await
            .expect("member");
    }
    id
}

pub async fn serve(state: AppState) -> SocketAddr {
    serve_with(state, |_| api::router::Options::default()).await
}

pub async fn serve_with(state: AppState, options: impl FnOnce(SocketAddr) -> api::router::Options) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let router = api::router::build_router(state, options(addr)).expect("router");
    let service = router.into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, service).await.expect("serve") });
    addr
}
