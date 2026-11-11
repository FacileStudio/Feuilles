use std::sync::Arc;

use porte::{Claims, StoreError, UserStore};

/// The account store with Registre's `picture` kept as `porte_users.avatar_url`.
///
/// porte-rs stores no avatars, so it always hands the store an empty `avatar_url`. Feuilles links
/// the picture Registre names instead, at every sign-in and profile sync (id token or userinfo),
/// when it is an `https` URL.
pub struct Pictures(pub Arc<dyn UserStore>);

#[async_trait::async_trait]
impl UserStore for Pictures {
    async fn upsert_from_oidc(&self, claims: &Claims) -> Result<i64, StoreError> {
        match claims.avatar_url.is_empty() && claims.picture.starts_with("https://") {
            true => self.0.upsert_from_oidc(&Claims { avatar_url: claims.picture.clone(), ..claims.clone() }).await,
            false => self.0.upsert_from_oidc(claims).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Seen(Mutex<Vec<String>>);

    #[async_trait::async_trait]
    impl UserStore for Seen {
        async fn upsert_from_oidc(&self, claims: &Claims) -> Result<i64, StoreError> {
            self.0.lock().expect("lock").push(claims.avatar_url.clone());
            Ok(1)
        }
    }

    #[tokio::test]
    async fn an_https_picture_becomes_the_avatar_and_nothing_else_does() {
        let seen = Arc::new(Seen::default());
        let store = Pictures(seen.clone());
        for picture in ["https://sso.facile.studio/avatars/7.png", "javascript:alert(1)", "http://x/a.png", ""] {
            store.upsert_from_oidc(&Claims { picture: picture.into(), ..Claims::default() }).await.expect("upsert");
        }
        assert_eq!(*seen.0.lock().expect("lock"), ["https://sso.facile.studio/avatars/7.png", "", "", ""]);
    }
}
