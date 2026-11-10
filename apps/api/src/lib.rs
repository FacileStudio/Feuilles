#![deny(unused, dead_code)]

//! The Feuilles API: workbooks on IronCalc, edited together over WebSocket.

pub mod auth;
pub mod engine;
pub mod env;
pub mod hub;
pub mod mcp;
pub mod modules;
pub mod purge;
pub mod reference;
pub mod router;
pub mod security;
pub mod serve;
pub mod state;
pub mod upgrade;
pub mod version;

/// The migrations, embedded in the binary.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
