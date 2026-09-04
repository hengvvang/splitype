use std::sync::Arc;
use std::time::Instant;

use markdown_parser::parse::BlockProjection;

fn main() {
    for (name, kb) in [("64KB", 64), ("256KB", 256), ("1MB", 1024), ("4MB", 4096)] {
        let mut text = String::new();
        while text.len() < kb * 1024 {
            text.push_str("# Heading\n\nSome paragraph with **bold** and `code`.\n\n- item one\n- item two\n\n> quote\n\n");
        }
        text.truncate(kb * 1024);

        // Full parse.
        let start = Instant::now();
        let mut projection = BlockProjection::parse(Arc::from(text.clone()));
        let elapsed = start.elapsed();
        println!(
            "{name}: {} blocks, full parse {elapsed:?}",
            projection.blocks.len()
        );

        // Incremental re-parse after one small mid-document edit.
        let mut edited = text.clone();
        let mid = edited.len() / 2;
        edited.replace_range(mid..mid + 8, "changed");
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, Arc::from(edited));
        let elapsed = start.elapsed();
        println!("  incremental re-parse (one word edit): {elapsed:?}");

        // Incremental re-parse after one line-level edit.
        let mut edited = text.clone();
        edited.replace_range(mid..mid + 8, "changed\n- extra item\n");
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, Arc::from(edited));
        let elapsed = start.elapsed();
        println!("  incremental re-parse (line + list item): {elapsed:?}");

        // Incremental re-parse opening a fenced block mid-document.
        let mut edited = text.clone();
        edited.replace_range(mid..mid + 8, "```\ncode\n```\n");
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, Arc::from(edited));
        let elapsed = start.elapsed();
        println!("  incremental re-parse (fence insertion): {elapsed:?}");
    }
}
