//! Boot: the steps of `run()` after the configuration is read.

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::auth::{Evict, Kits};
use crate::env::Config;
use crate::router::{self, Options};
use crate::state::AppState;
use crate::{MIGRATOR, purge, security};

/// How long in-flight requests get after a shutdown signal.
pub const DRAIN: Duration = Duration::from_secs(10);

/// Logs a boot failure and turns it into the exit code.
pub fn fail(step: &str, error: &dyn std::fmt::Display) -> ExitCode {
    tracing::error!(%error, "feuilles: {step} failed");
    eprintln!("feuilles: {step}: {error}");
    ExitCode::FAILURE
}

/// Steps 3 to 9 of the boot: database, migrations, kits, background tasks, router, serve.
pub async fn run(config: Config, args: Vec<String>) -> ExitCode {
    let pool = match PgPool::connect(&config.core.database_url).await {
        Ok(pool) => pool,
        Err(error) => return fail("open the database", &error),
    };
    if let Some(result) = tronc::migrate::command(&args, &pool, &MIGRATOR).await {
        return result.map_or_else(|e| fail("migrate", &e), |()| ExitCode::SUCCESS);
    }
    if let Err(error) = tronc::migrate::run(&pool, &MIGRATOR).await {
        return fail("migrate", &error);
    }
    let evict: Evict = Arc::new(|evictions| tracing::info!(count = evictions.len(), "directory: access removed"));
    let kits = match Kits::new(&config.porte, &pool, &config.token_key, evict).await {
        Ok(kits) => kits,
        Err(error) => return fail("build the auth kits", &error),
    };
    let state = AppState::new(pool, config.porte.issuer.clone(), kits);
    let shutdown = CancellationToken::new();
    spawn_tasks(&state, &config, &shutdown);
    let mut http = tronc::http::Config::from_core(&config.core);
    http.security = security::config(&config.client_dir);
    let app = match router::build_router(state, Options { http, client_dir: config.client_dir.clone() }) {
        Ok(app) => app,
        Err(error) => return fail("build the router", &error),
    };
    listen(app, config.core.port, shutdown).await
}

fn spawn_tasks(state: &AppState, config: &Config, shutdown: &CancellationToken) {
    tokio::spawn(purge::run(state.clone(), shutdown.clone()));
    if !config.porte.enabled() {
        tracing::warn!("OIDC_ISSUER is not set: no sign-in, no directory sync, no deletions");
        return;
    }
    let (syncer, deletions) = (state.kits.syncer.clone(), state.kits.deletions.clone());
    let (one, two) = (shutdown.clone(), shutdown.clone());
    tokio::spawn(async move { syncer.run(one).await });
    tokio::spawn(async move { deletions.run(two).await });
}

async fn listen(app: axum::Router, port: u16, shutdown: CancellationToken) -> ExitCode {
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => listener,
        Err(error) => return fail("listen", &error),
    };
    tracing::info!(port, "feuilles: listening");
    let drained = shutdown.clone();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let server = axum::serve(listener, service).with_graceful_shutdown(async move { drained.cancelled().await });
    let stopping = shutdown.clone();
    tokio::spawn(async move {
        signal().await;
        stopping.cancel();
    });
    let deadline = async move {
        shutdown.cancelled().await;
        tokio::time::sleep(DRAIN).await;
    };
    tokio::select! {
        result = server.into_future() => result.map_or_else(|e| fail("serve", &e), |()| ExitCode::SUCCESS),
        () = deadline => {
            tracing::warn!("feuilles: connections still open after the drain, exiting");
            ExitCode::SUCCESS
        }
    }
}

async fn signal() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(term) => term,
            Err(_) => return interrupt.await.unwrap_or_default(),
        };
        tokio::select! { _ = interrupt => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    interrupt.await.unwrap_or_default();
}
