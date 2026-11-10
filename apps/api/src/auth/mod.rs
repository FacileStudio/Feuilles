//! porte wiring: sessions, Registre sign-in, the directory sync and account deletions.

mod deletions;
mod members;
mod retire;
mod routes;
mod spaces;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use porte::directory::{self, Deletions, Syncer};
use porte::http_client::HttpClient;
use porte::oidc::{self, Kit};
use porte::pg::{self, TokenSealer};
use porte::session::{self, Manager};
use sqlx::PgPool;

pub use deletions::{AppData, DeletionState, erase_person};
pub use members::{Members, subject_of};
pub use routes::{Me, SessionAnswer, routes};
pub use spaces::{Evict, Eviction, SpacesApply, apply_snapshot, local_role};

/// Every porte kit the app runs, built once at boot.
#[derive(Clone)]
pub struct Kits {
    /// Sessions and the auth middleware.
    pub sessions: Manager,
    /// Registre sign-in.
    pub oidc: Kit,
    /// The directory sync loop and its nudge route.
    pub syncer: Syncer,
    /// The account deletion loop and its push route.
    pub deletions: Deletions,
    /// porte's tables.
    pub store: pg::Store,
}

impl Kits {
    /// Builds the kits. With an issuer this runs OIDC discovery, so a wrong issuer fails the
    /// boot. `evict` hears about everyone who lost access to a workbook.
    pub async fn new(
        cfg: &porte::Config,
        pool: &PgPool,
        token_key: &str,
        evict: Evict,
    ) -> Result<Kits, Box<dyn Error + Send + Sync>> {
        let store = pg::Store::new(pool.clone()).with_token_sealer(TokenSealer::parse(token_key)?);
        let sessions = Manager::new(cfg, session::Deps { sessions: store.sessions(), now: None });
        let deps = oidc::Deps {
            users: store.users(),
            identities: store.identities(),
            codes: store.login_codes(),
            sessions: sessions.clone(),
            config_extra: None,
            http: None,
            now: None,
        };
        let oidc = Kit::new(cfg, deps).await?;
        let client = directory::Client {
            issuer: cfg.issuer.clone(),
            client_id: cfg.client_id.clone(),
            client_secret: cfg.client_secret.clone(),
            http: HttpClient::new(),
        };
        let apply = Arc::new(SpacesApply::new(pool.clone(), evict.clone()));
        let syncer = Syncer::new(client.clone(), apply, Duration::ZERO);
        let erase = Arc::new(AppData::new(pool.clone(), evict));
        let state = Arc::new(DeletionState::new(pool.clone()));
        let deletions = Deletions::new(client, erase, Some(Arc::new(store.clone())), Some(state), Duration::ZERO);
        Ok(Kits { sessions, oidc, syncer, deletions, store })
    }
}
