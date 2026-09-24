use std::process::ExitCode;

use clap::Parser;
use feedrsauros::cli::{Cli, run};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // Logs go to stderr so stdout stays clean for output like `feedrsauros export > file.opml`.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("feedrsauros=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
