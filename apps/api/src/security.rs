use std::path::Path;

use tronc::http::{SecurityConfig, Sources};

/// tronc's policy plus what the Feuilles client needs: `wasm-unsafe-eval` to compile the
/// IronCalc engine, and the hashes of the inline scripts adapter-static writes. The policy stays
/// report-only, like every suite app's.
pub fn config(client_dir: &Path) -> SecurityConfig {
    let mut scripts = vec!["'wasm-unsafe-eval'".to_owned()];
    if tronc::spa::available(client_dir) {
        match tronc::spa::inline_script_hashes(client_dir) {
            Ok(hashes) => scripts.extend(hashes),
            Err(error) => tracing::error!(%error, "failed to hash the client's inline scripts"),
        }
    }
    SecurityConfig { csp_sources: Sources::from([("script-src".to_owned(), scripts)]), ..SecurityConfig::default() }
}
