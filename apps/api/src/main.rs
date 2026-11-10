use std::process::ExitCode;

fn main() -> ExitCode {
    if tronc::healthcheck::handle(std::env::args()) {
        return ExitCode::SUCCESS;
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map(|runtime| runtime.block_on(run()))
        .unwrap_or(ExitCode::FAILURE)
}

async fn run() -> ExitCode {
    let config = match api::env::Config::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("feuilles: {error}");
            return ExitCode::FAILURE;
        }
    };
    let _logger = tronc::logger::init(tronc::logger::Config::from_core(&config.core));
    api::serve::run(config, std::env::args().collect()).await
}
