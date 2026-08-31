//! Matching bracket detection.

/// Finds the matching bracket offset for a bracket at `cursor_offset`.
pub fn find_matching_bracket(text: &str, cursor_offset: usize) -> Option<usize> {
    if text.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();
    let check_offset = if cursor_offset < bytes.len() && is_bracket(bytes[cursor_offset]) {
        cursor_offset
    } else if cursor_offset > 0 && is_bracket(bytes[cursor_offset - 1]) {
        cursor_offset - 1
    } else {
        return None;
    };

    let target_byte = bytes[check_offset];
    let (match_byte, forward) = match target_byte {
        b'(' => (b')', true),
        b'[' => (b']', true),
        b'{' => (b'}', true),
        b')' => (b'(', false),
        b']' => (b'[', false),
        b'}' => (b'{', false),
        _ => return None,
    };

    let mut depth = 0;
    if forward {
        for (idx, &b) in bytes.iter().enumerate().skip(check_offset) {
            if b == target_byte {
                depth += 1;
            } else if b == match_byte {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
        }
    } else {
        for idx in (0..=check_offset).rev() {
            let b = bytes[idx];
            if b == target_byte {
                depth += 1;
            } else if b == match_byte {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
        }
    }

    None
}

fn is_bracket(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'[' | b']' | b'{' | b'}')
}

