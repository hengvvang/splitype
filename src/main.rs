//! Binary entry point — delegates to the `splitype` library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args = splitype::app::cli::parse();
    splitype::app::bootstrap::run(args);
}
