//! splitype - a block-based Markdown editor built with GPUI.
//!
//! Reads file paths from command-line arguments and opens one GPUI window per
//! file. With no arguments, a single empty window is created.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod infra;
mod layout;
mod model;
mod platform;
mod theme;
mod ui;
mod windows;

fn main() {
    let args = app::cli::parse();
    app::bootstrap::run(args);
}
