use std::path::PathBuf;

use tronc::env::{Core, EnvError, Vars};

/// The Bulle the client frames when `BULLE_URL` is unset.
pub const DEFAULT_BULLE: &str = "https://bulle.facile.studio";

/// Everything the API reads from the environment, read once at boot.
#[derive(Debug, Clone)]
pub struct Config {
    /// The variables every suite API reads, from tronc.
    pub core: Core,
    /// The porte configuration: Registre sign-in, sessions. `sso_only` is always on.
    pub porte: porte::Config,
    /// `CLIENT_DIR`, the built SPA.
    pub client_dir: PathBuf,
    /// `PORTE_TOKEN_KEY`, the key that seals stored OIDC tokens.
    pub token_key: String,
    /// `MCP_RESOURCE`, the public URL of `/mcp`. Empty turns MCP off.
    pub mcp_resource: String,
    /// `BULLE_URL`, the origin of the Bulle sidebar the client frames and the CSP allows.
    pub bulle: String,
}

impl Config {
    /// Reads the process environment.
    pub fn load() -> Result<Config, EnvError> {
        Config::from_vars(&Vars::process())
    }

    /// Reads `vars`, and refuses a production boot that lacks what production needs.
    pub fn from_vars(vars: &Vars) -> Result<Config, EnvError> {
        let core = Core::from_vars(vars, true)?;
        let mut porte = porte::Config::from_vars(vars)?;
        porte.sso_only = true;
        let config = Config {
            core,
            porte,
            client_dir: PathBuf::from(vars.string("CLIENT_DIR", tronc::spa::DEFAULT_DIR)),
            token_key: vars.string("PORTE_TOKEN_KEY", "").trim().to_owned(),
            mcp_resource: vars.string("MCP_RESOURCE", "").trim().to_owned(),
            bulle: crate::security::bulle_origin(&vars.string("BULLE_URL", "")).map_err(EnvError)?,
        };
        config.check()?;
        Ok(config)
    }

    fn check(&self) -> Result<(), EnvError> {
        if !self.mcp_resource.is_empty() && !self.porte.enabled() {
            return Err(EnvError("MCP_RESOURCE needs OIDC_ISSUER".into()));
        }
        if !self.mcp_resource.is_empty() && self.token_key.is_empty() {
            return Err(EnvError("MCP_RESOURCE needs PORTE_TOKEN_KEY, which keys its confirmations".into()));
        }
        if !self.core.is_production() {
            return Ok(());
        }
        let missing = [("OIDC_ISSUER", &self.porte.issuer), ("PORTE_TOKEN_KEY", &self.token_key)]
            .into_iter()
            .find(|(_, value)| value.is_empty());
        match missing {
            Some((key, _)) => Err(EnvError(format!("{key} is required in production"))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(extra: &[(&str, &str)]) -> Vars {
        let mut pairs = vec![("DATABASE_URL", "postgres://localhost/feuilles")];
        pairs.extend_from_slice(extra);
        Vars::from_pairs(pairs)
    }

    #[test]
    fn development_boots_without_sso_and_forces_sso_only() {
        let config = Config::from_vars(&vars(&[])).expect("development config");
        assert!(config.porte.sso_only);
        assert!(!config.porte.enabled());
    }

    #[test]
    fn production_refuses_to_boot_without_issuer_or_token_key() {
        let error = Config::from_vars(&vars(&[("APP_ENV", "production")])).unwrap_err();
        assert!(error.0.contains("OIDC_ISSUER"), "{}", error.0);
        let issuer = [("APP_ENV", "production"), ("OIDC_ISSUER", "https://sso.facile.studio")];
        let error = Config::from_vars(&vars(&issuer)).unwrap_err();
        assert!(error.0.contains("PORTE_TOKEN_KEY"), "{}", error.0);
    }

    #[test]
    fn mcp_needs_an_issuer_and_a_token_key() {
        let mcp = ("MCP_RESOURCE", "https://feuilles.facile.studio/mcp");
        let error = Config::from_vars(&vars(&[mcp])).unwrap_err();
        assert!(error.0.contains("OIDC_ISSUER"), "{}", error.0);
        let error = Config::from_vars(&vars(&[mcp, ("OIDC_ISSUER", "https://sso.facile.studio")])).unwrap_err();
        assert!(error.0.contains("PORTE_TOKEN_KEY"), "{}", error.0);
    }
}
