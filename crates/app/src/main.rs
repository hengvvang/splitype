//! Binary entry point — delegates to the `splitype` library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "info,fontdb=error,usvg=error,gpui::platform::windows::direct_write=off",
        )
    });

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    let args = splitype_cli::parse();
    app::bootstrap::run(args);
}

