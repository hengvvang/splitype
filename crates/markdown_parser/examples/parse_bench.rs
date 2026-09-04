use std::time::Instant;

use markdown_parser::parse::BlockProjection;
use rope::Rope;

/// Changed lines of `old` after the byte diff to `new`, in old coordinates
/// (mirrors the buffer's derivation from its edit ranges).
fn changed_lines(old: &str, new: &str) -> (usize, usize) {
    let (prefix, suffix) = markdown_parser::parse::common_affix(old, new);
    let line_of = |byte: usize| {
        old.as_bytes()[..byte.min(old.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
    };
    let changed_start = prefix;
    let changed_end = old.len() - suffix;
    let count = old.lines().count().max(1);
    let first = line_of(changed_start).min(count - 1);
    let last = if changed_end > changed_start {
        line_of(changed_end)
            .min(count - 1)
            .max(line_of(changed_end - 1))
    } else {
        first
    };
    (first, last)
}

fn main() {
    for (name, kb) in [("64KB", 64), ("256KB", 256), ("1MB", 1024), ("4MB", 4096)] {
        let mut text = String::new();
        while text.len() < kb * 1024 {
            text.push_str("# Heading\n\nSome paragraph with **bold** and `code`.\n\n- item one\n- item two\n\n> quote\n\n");
        }
        text.truncate(kb * 1024);
        let rope = Rope::new(&text);

        // Full parse.
        let start = Instant::now();
        let mut projection = BlockProjection::parse(&rope);
        let elapsed = start.elapsed();
        println!(
            "{name}: {} blocks, full parse {elapsed:?}",
            projection.blocks.len()
        );

        // Incremental re-parse after one small mid-document edit.
        let mut edited = text.clone();
        let mid = edited.len() / 2;
        edited.replace_range(mid..mid + 8, "changed");
        let new_rope = Rope::new(&edited);
        let (first, last) = changed_lines(&text, &edited);
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        let elapsed = start.elapsed();
        println!("  incremental re-parse (one word edit): {elapsed:?}");

        // Incremental re-parse after one line-level edit.
        let mut edited = text.clone();
        edited.replace_range(mid..mid + 8, "changed\n- extra item\n");
        let new_rope = Rope::new(&edited);
        let (first, last) = changed_lines(&text, &edited);
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        let elapsed = start.elapsed();
        println!("  incremental re-parse (line + list item): {elapsed:?}");

        // Incremental re-parse opening a fenced block mid-document.
        let mut edited = text.clone();
        edited.replace_range(mid..mid + 8, "```\ncode\n```\n");
        let new_rope = Rope::new(&edited);
        let (first, last) = changed_lines(&text, &edited);
        let start = Instant::now();
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        let elapsed = start.elapsed();
        println!("  incremental re-parse (fence insertion): {elapsed:?}");
    }
}
