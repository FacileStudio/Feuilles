use std::sync::Arc;

use porte::spaces::Guard;
use sqlx::PgPool;

use crate::auth::{Kits, Members};
use crate::hub::Hub;

/// What every handler shares.
#[derive(Clone)]
pub struct AppState {
    /// The database.
    pub pool: PgPool,
    /// `OIDC_ISSUER`: the provider of the identities whose subjects key space membership.
    pub issuer: String,
    /// The porte kits.
    pub kits: Kits,
    /// Space membership, over `space_members`.
    pub guard: Guard,
    /// The collaboration rooms.
    pub hub: Hub,
}

impl AppState {
    /// The state over `pool`, with membership answered from the app's own tables.
    pub fn new(pool: PgPool, issuer: String, kits: Kits, hub: Hub) -> AppState {
        let guard = Guard::new(Arc::new(Members::new(pool.clone())));
        AppState { pool, issuer, kits, guard, hub }
    }
}
