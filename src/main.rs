//! Binary entry point — delegates to the `splitype` library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let args = splitype::app::cli::parse();
    splitype::app::bootstrap::run(args);
}
