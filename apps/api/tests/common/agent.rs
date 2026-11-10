use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use api::mcp::Mcp;
use serde_json::Value;
use tronc::mcp::{TokenInfo, Verifier, VerifyError, VerifyFuture};
use tronc::mcptest::{self, ClientOptions};

use super::serve_with;

pub struct DevTokens;

impl Verifier for DevTokens {
    fn verify<'a>(&'a self, token: &'a str, _request: &'a http::request::Parts) -> VerifyFuture<'a> {
        Box::pin(async move {
            let (user, scopes) = token.split_once('|').ok_or(VerifyError::Invalid)?;
            let scopes = scopes.split(',').map(str::to_owned).collect();
            let extra = HashMap::from([(tronc::mcp::EXTRA_CLIENT_ID.to_owned(), "test-client".to_owned())]);
            Ok(TokenInfo {
                user_id: user.into(),
                scopes,
                expires_at: Some(SystemTime::now() + Duration::from_secs(600)),
                extra,
            })
        })
    }
}

pub async fn mounted(state: &api::state::AppState) -> SocketAddr {
    serve_with(state.clone(), |addr| api::router::Options {
        mcp: Some(Mcp {
            resource: format!("http://{addr}/mcp"),
            issuer: super::ISSUER.into(),
            verifier: Arc::new(DevTokens),
        }),
        ..Default::default()
    })
    .await
}

pub fn args(value: Value) -> tronc::mcp::rmcp::model::JsonObject {
    value.as_object().expect("object").clone()
}

pub async fn token(addr: SocketAddr, user: i64, scopes: &str) -> mcptest::Client {
    mcptest::connect(&format!("http://{addr}/mcp"), &format!("{user}|{scopes}"), ClientOptions::default()).await
}
