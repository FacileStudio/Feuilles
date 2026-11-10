use api::auth::Kits;
use api::hub::{Hub, Limits};
use api::modules::live::evictor;
use api::state::AppState;

use super::ISSUER;

pub async fn state_in(prefix: &str, limits: Limits) -> Option<AppState> {
    let pool = tronc::testdb::pool(prefix, &api::MIGRATOR).await?;
    let hub = Hub::with_limits(pool.clone(), tokio::runtime::Handle::current(), 2, limits);
    let kits = Kits::new(&porte::Config::default(), &pool, "", evictor(hub.clone())).await.expect("kits");
    Some(AppState::new(pool, ISSUER.to_owned(), kits, hub))
}
