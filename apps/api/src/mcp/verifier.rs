use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use http::request::Parts;
use porte::mcpauth::{self, VerifyError as PorteError};
use tronc::mcp::{EXTRA_ACTOR, EXTRA_CLIENT_ID, TokenInfo, Verifier, VerifyError, VerifyFuture};

use crate::auth::Kits;

/// `porte::mcpauth::Verifier` behind tronc's `Verifier` trait.
pub struct PorteVerifier(pub Arc<mcpauth::Verifier>);

impl Verifier for PorteVerifier {
    fn verify<'a>(&'a self, token: &'a str, _request: &'a Parts) -> VerifyFuture<'a> {
        Box::pin(async move {
            let info = self.0.verify(token).await.map_err(|error| match error {
                PorteError::Unavailable => VerifyError::Unavailable,
                _ => VerifyError::Invalid,
            })?;
            let expires = SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(info.expiration.unix_timestamp().max(0).unsigned_abs());
            let extra =
                HashMap::from([(EXTRA_CLIENT_ID.to_owned(), info.client_id), (EXTRA_ACTOR.to_owned(), info.actor)]);
            Ok(TokenInfo { user_id: info.user_id, scopes: info.scopes, expires_at: Some(expires), extra })
        })
    }
}

/// The verifier for `resource`: Registre's tokens, accounts provisioned on a first call.
pub async fn porte_verifier(
    cfg: &porte::Config,
    kits: &Kits,
    resource: &str,
) -> Result<PorteVerifier, porte::ConfigError> {
    let verifier = mcpauth::Verifier::new(mcpauth::Config {
        issuer: cfg.issuer.clone(),
        resource: resource.to_owned(),
        scope_prefix: "feuilles".into(),
        audience: cfg.client_id.clone(),
        identities: kits.store.identities(),
        provision: true,
        accounts: Some(kits.oidc.clone()),
        http: porte::http_client::HttpClient::new(),
    })
    .await?;
    Ok(PorteVerifier(Arc::new(verifier)))
}
