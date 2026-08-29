//! Command-line argument parsing.

use std::path::PathBuf;

pub(crate) mod install;

/// Parsed command-line arguments.
pub struct Args {
    pub detach: bool,
    pub input_paths: Vec<PathBuf>,
}

/// Parse command-line arguments, printing help/version and exiting when
/// requested.
pub fn parse() -> Args {
    let args: Vec<String> = std::env::args().collect();

    let mut detach = false;
    let mut input_paths = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-v" => {
                println!("splitype {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--detach" | "-d" => {
                detach = true;
            }
            option if option.starts_with('-') => {
                eprintln!("Unknown option: {}", option);
                std::process::exit(1);
            }
            path => {
                input_paths.push(PathBuf::from(path));
            }
        }
        i += 1;
    }

    Args {
        detach,
        input_paths,
    }
}

fn print_help() {
    println!(
        "splitype {} - A block-based Markdown editor",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    splitype [OPTIONS] [FILES...]");
    println!();
    println!("OPTIONS:");
    println!("    -v, --version    Print version information");
    println!("    -h, --help       Print this help message");
    println!("    -d, --detach     Launch in background (non-blocking)");
    println!();
    println!("FILES:");
    println!("    One or more markdown files to open. If no files are specified,");
    println!("    opens an empty document.");
}
