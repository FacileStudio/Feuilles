#![allow(dead_code)]

use std::sync::Arc;

use api::auth::{Evict, Kits};
use api::router::{self, Options};
use api::state::AppState;
use axum::Router;
use uuid::Uuid;

pub mod web;

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
    let pool = tronc::testdb::pool("feuilles", &api::MIGRATOR).await?;
    let evict: Evict = Arc::new(|_| {});
    let kits = Kits::new(&porte::Config::default(), &pool, "", evict).await.expect("kits");
    Some(AppState::new(pool, ISSUER.to_owned(), kits))
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
