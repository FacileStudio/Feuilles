//! Test-only seeding for the browser end-to-end run (`apps/client/e2e`). Never shipped: the
//! image builds the `api` binary only. It writes two space members and one outsider straight
//! into a scratch database, with Registre identities under `OIDC_ISSUER`, and mints each a
//! browser session through porte's own session store, so its token carries
//! `porte::BROWSER_TOKEN_PREFIX` and confirms agent actions like a real login. It prints the
//! cookies as JSON.
//!
//! `DATABASE_URL=... OIDC_ISSUER=... cargo run --example e2e_seed`

use std::process::ExitCode;

use porte::pg::Store;
use porte::session::{Deps, Manager};
use sqlx::PgPool;

const SPACE: &str = "e2e-space";

#[tokio::main]
async fn main() -> ExitCode {
    match seed().await {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("e2e_seed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn seed() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")?;
    let issuer = std::env::var("OIDC_ISSUER")?;
    let pool = PgPool::connect(&url).await?;
    let config = porte::Config { issuer: issuer.clone(), ..porte::Config::default() };
    let sessions = Manager::new(&config, Deps { sessions: Store::new(pool.clone()).sessions(), now: None });
    sqlx::query("INSERT INTO spaces (id, name) VALUES ($1, 'Shared')").bind(SPACE).execute(&pool).await?;
    let mut cookies = serde_json::Map::new();
    for (name, role) in [("alice", Some("owner")), ("bob", Some("member")), ("outsider", None)] {
        let user = person(&pool, &issuer, name, role).await?;
        let template = porte::Session { user_id: user, ..porte::Session::default() };
        let (token, _, _) = sessions.issue_cookie_for(&http::Request::new(()), template).await?;
        cookies.insert(name.into(), token.into());
    }
    Ok(serde_json::json!({ "space": SPACE, "cookies": cookies }))
}

async fn person(pool: &PgPool, issuer: &str, name: &str, role: Option<&str>) -> Result<i64, sqlx::Error> {
    let subject = format!("e2e-{name}");
    let email = format!("{name}@e2e.test");
    let user: i64 = sqlx::query_scalar("INSERT INTO porte_users (email, name) VALUES ($1, $2) RETURNING id")
        .bind(&email)
        .bind(name)
        .fetch_one(pool)
        .await?;
    sqlx::query("INSERT INTO porte_identities (user_id, provider, subject) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(issuer)
        .bind(&subject)
        .execute(pool)
        .await?;
    if let Some(role) = role {
        sqlx::query("INSERT INTO space_members (space_id, subject, email, name, role) VALUES ($1, $2, $3, $4, $5)")
            .bind(SPACE)
            .bind(&subject)
            .bind(&email)
            .bind(name)
            .bind(role)
            .execute(pool)
            .await?;
    }
    Ok(user)
}
