//! splitype_cli — the command-line entry contract of the Splitype editor.
//!
//! Owns nothing but the CLI argument surface: the parsed [`Args`] and the
//! `parse()`/`--help` handling. Pure `std` — the app composition root
//! reads the parsed arguments and boots its windows from them; the
//! macOS CLI-tool installation wizard lives in the app (it needs gpui
//! windows, i18n and platform operations).

use std::path::PathBuf;

/// Parsed command-line arguments.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Args {
    /// Launch detached from the terminal (non-blocking).
    pub detach: bool,
    /// Markdown files to open; empty means "open an empty document".
    pub input_paths: Vec<PathBuf>,
}

/// Parse command-line arguments, printing help/version and exiting when
/// requested.
pub fn parse() -> Args {
    let args: Vec<String> = std::env::args().collect();
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--version" | "-v" => {
                println!("splitype {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
    }
    parse_from(&args[1..])
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

/// Parse the given argument vector (without the program name) — the
/// testable core of [`parse`].
fn parse_from(args: &[String]) -> Args {
    let mut detach = false;
    let mut input_paths = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--detach" | "-d" => detach = true,
            option if option.starts_with('-') => {
                eprintln!("Unknown option: {}", option);
                std::process::exit(1);
            }
            path => input_paths.push(PathBuf::from(path)),
        }
    }
    Args {
        detach,
        input_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_args_open_nothing_detached() {
        let parsed = parse_from(&[]);
        assert!(!parsed.detach);
        assert!(parsed.input_paths.is_empty());
    }

    #[test]
    fn collects_file_paths_in_order() {
        let parsed = parse_from(&args(&["a.md", "b.md", "c.md"]));
        assert!(!parsed.detach);
        assert_eq!(
            parsed.input_paths,
            vec![
                PathBuf::from("a.md"),
                PathBuf::from("b.md"),
                PathBuf::from("c.md")
            ]
        );
    }

    #[test]
    fn detach_flag_is_recognized_in_any_position() {
        let parsed = parse_from(&args(&["--detach", "a.md"]));
        assert!(parsed.detach);
        assert_eq!(parsed.input_paths, vec![PathBuf::from("a.md")]);

        let parsed = parse_from(&args(&["a.md", "-d"]));
        assert!(parsed.detach);
    }

    #[test]
    fn flags_and_paths_mix() {
        let parsed = parse_from(&args(&["-d", "one.md", "--detach", "two.md"]));
        assert!(parsed.detach);
        assert_eq!(
            parsed.input_paths,
            vec![PathBuf::from("one.md"), PathBuf::from("two.md")]
        );
    }

    #[test]
    fn paths_may_be_relative_or_absolute() {
        let parsed = parse_from(&args(&["C:\\docs\\a.md", "relative.md"]));
        assert_eq!(
            parsed.input_paths,
            vec![PathBuf::from("C:\\docs\\a.md"), PathBuf::from("relative.md")]
        );
    }
}
