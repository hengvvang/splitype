//! Indentation guide lines calculation.

/// Computes the indentation levels (in columns) for a given line.
pub fn compute_indent_guide_columns(line: &str, tab_size: u32) -> Vec<u32> {
    let tab_size = tab_size.max(1);
    let mut leading_spaces = 0;
    for ch in line.chars() {
        if ch == ' ' {
            leading_spaces += 1;
        } else if ch == '\t' {
            leading_spaces += tab_size;
        } else {
            break;
        }
    }

    let mut guides = Vec::new();
    let count = leading_spaces / tab_size;
    for i in 1..=count {
        guides.push((i - 1) * tab_size);
    }
    guides
}
