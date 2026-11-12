mod common;

use std::sync::Arc;

use api::auth::Kits;
use axum::Json;
use axum::extract::State;
use axum::routing::get;
use porte::jwt::{Claims, Profile};
use porte::oidc::ProvisionError;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_person_cannot_sign_in_again_and_a_recreated_account_is_erased_again() {
    let Some(pool) = tronc::testdb::pool("feuilles", &api::MIGRATOR).await else { return };
    let subject = common::unique("sub");
    let issuer = registre(&subject).await;
    let cfg = porte::Config {
        issuer: issuer.clone(),
        client_id: "feuilles".into(),
        client_secret: "secret".into(),
        redirect_url: "https://feuilles.test/api/auth/oidc/callback".into(),
        success_url: "https://feuilles.test/".into(),
        ..porte::Config::default()
    };
    let kits = Kits::new(&cfg, &pool, "", Arc::new(|_| {})).await.expect("kits");
    kits.oidc.provision_bearer(&token(&subject)).await.expect("first sign-in");
    assert!(holds(&pool, &issuer, &subject).await);
    kits.deletions.poll().await.expect("poll");
    assert!(!holds(&pool, &issuer, &subject).await, "the poll erases the person");
    let refused = kits.oidc.provision_bearer(&token(&subject)).await;
    assert!(matches!(refused, Err(ProvisionError::Erased)), "an erased subject signed in again: {refused:?}");
    assert!(!holds(&pool, &issuer, &subject).await);
    recreate(&pool, &issuer, &subject).await;
    kits.deletions.poll().await.expect("poll again");
    assert!(!holds(&pool, &issuer, &subject).await, "a re-created account survived the next poll");
    let other = common::unique("sub");
    kits.oidc.provision_bearer(&token(&other)).await.expect("someone else still signs in");
}

async fn registre(deleted: &str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let issuer = format!("http://{}", listener.local_addr().expect("addr"));
    let state = (issuer.clone(), deleted.to_owned());
    let app = axum::Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/keys", get(|| async { Json(json!({ "keys": [] })) }))
        .route("/api/directory/deletions", get(deletions))
        .with_state(Arc::new(state));
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    issuer
}

async fn discovery(State(state): State<Arc<(String, String)>>) -> Json<Value> {
    let issuer = &state.0;
    Json(json!({
        "issuer": issuer,
        "jwks_uri": format!("{issuer}/keys"),
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "userinfo_endpoint": format!("{issuer}/userinfo"),
    }))
}

async fn deletions(State(state): State<Arc<(String, String)>>) -> Json<Value> {
    Json(json!({ "deleted": [{ "sub": state.1, "deleted_at": "2026-11-12T10:00:00Z" }] }))
}

fn token(subject: &str) -> Claims {
    Claims {
        subject: subject.into(),
        email: format!("{subject}@example.test"),
        name: subject.into(),
        roles: None,
        scopes: vec!["feuilles.read".into()],
        actor: String::new(),
        audience: vec!["feuilles".into()],
        client_id: "claude".into(),
        id: String::new(),
        expires_at: time::OffsetDateTime::now_utc(),
        profile: Profile { email_verified: Some(true), ..Profile::default() },
    }
}

async fn holds(pool: &sqlx::PgPool, issuer: &str, subject: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM porte_identities WHERE provider = $1 AND subject = $2)")
        .bind(issuer)
        .bind(subject)
        .fetch_one(pool)
        .await
        .expect("identity")
}

async fn recreate(pool: &sqlx::PgPool, issuer: &str, subject: &str) {
    let user: i64 = sqlx::query_scalar("INSERT INTO porte_users (email, name) VALUES ($1, $1) RETURNING id")
        .bind(format!("{subject}@example.test"))
        .fetch_one(pool)
        .await
        .expect("user");
    sqlx::query("INSERT INTO porte_identities (user_id, provider, subject) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(issuer)
        .bind(subject)
        .execute(pool)
        .await
        .expect("identity");
}
