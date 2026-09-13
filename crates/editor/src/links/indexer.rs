//! Document link extraction and workspace backlink indexing.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::types::{BacklinkGroup, BacklinkMention, OutgoingLinkItem};

/// Extracts outgoing links (both internal Wikilinks and external URLs) from Markdown text.
pub fn extract_outgoing_links(text: &str) -> Vec<OutgoingLinkItem> {
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // 1. Check for Wikilink: `[[...]]` or `![[...]]`
        if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i + 2;
            if let Some(end_rel) = text[start..].find("]]") {
                let inner = &text[start..start + end_rel].trim();
                if !inner.is_empty() {
                    let (raw_target, alias) = if let Some((t, a)) = inner.split_once('|') {
                        (t.trim(), Some(a.trim()))
                    } else {
                        (*inner, None)
                    };

                    let (file_or_head, anchor) = if let Some((f, a)) = raw_target.split_once('#') {
                        (f.trim(), Some(a.trim().to_string()))
                    } else {
                        (raw_target, None)
                    };

                    let display_text = if let Some(a) = alias {
                        a.to_string()
                    } else if !file_or_head.is_empty() {
                        if let Some(ref anc) = anchor {
                            format!("{file_or_head} > {anc}")
                        } else {
                            file_or_head.to_string()
                        }
                    } else if let Some(ref anc) = anchor {
                        format!("#{anc}")
                    } else {
                        raw_target.to_string()
                    };

                    let full_target = format!("wikilink:{raw_target}");
                    if seen.insert(full_target.clone()) {
                        items.push(OutgoingLinkItem {
                            display_text,
                            target: full_target,
                            is_external: false,
                            anchor,
                        });
                    }
                }
                i = start + end_rel + 2;
                continue;
            }
        }

        // 2. Check for standard Markdown link: `[text](url)`
        if bytes[i] == b'[' {
            let text_start = i + 1;
            if let Some(text_end_rel) = text[text_start..].find(']') {
                let text_end = text_start + text_end_rel;
                if text_end + 1 < len && bytes[text_end + 1] == b'(' {
                    let url_start = text_end + 2;
                    if let Some(url_end_rel) = text[url_start..].find(')') {
                        let url_end = url_start + url_end_rel;
                        let link_label = text[text_start..text_end].trim();
                        let link_target = text[url_start..url_end].trim();

                        if !link_target.is_empty() && !link_target.starts_with('#') {
                            let is_ext = link_target.starts_with("http://")
                                || link_target.starts_with("https://")
                                || link_target.starts_with("mailto:");

                            let (clean_target, anchor) = if is_ext {
                                (link_target.to_string(), None)
                            } else if let Some((f, a)) = link_target.split_once('#') {
                                (f.trim().to_string(), Some(a.trim().to_string()))
                            } else {
                                (link_target.to_string(), None)
                            };

                            let display = if !link_label.is_empty() {
                                link_label.to_string()
                            } else {
                                clean_target.clone()
                            };

                            if seen.insert(clean_target.clone()) {
                                items.push(OutgoingLinkItem {
                                    display_text: display,
                                    target: clean_target,
                                    is_external: is_ext,
                                    anchor,
                                });
                            }
                        }
                        i = url_end + 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    items
}

/// Recursively scans `workspace_root` for `.md` files that reference `target_file_path`.
pub fn scan_workspace_backlinks(
    workspace_root: &Path,
    target_file_path: &Path,
) -> Vec<BacklinkGroup> {
    let mut groups = Vec::new();

    let target_stem = if let Some(stem) = target_file_path.file_stem() {
        stem.to_string_lossy().to_string()
    } else {
        return groups;
    };

    let target_canonical = target_file_path.canonicalize().ok();
    let target_stem_lower = target_stem.to_lowercase();
    let target_filename = target_file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{target_stem}.md"));
    let target_filename_lower = target_filename.to_lowercase();

    let mut dir_stack = vec![workspace_root.to_path_buf()];

    while let Some(dir) = dir_stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden directories and build outputs
            if path.is_dir() {
                if name.starts_with('.')
                    || name.eq_ignore_ascii_case("target")
                    || name.eq_ignore_ascii_case("node_modules")
                    || name.eq_ignore_ascii_case("dist")
                    || name.eq_ignore_ascii_case("build")
                {
                    continue;
                }
                dir_stack.push(path);
                continue;
            }

            if !name.ends_with(".md") && !name.ends_with(".markdown") {
                continue;
            }

            // Do not scan target file itself
            if let (Some(tc), Ok(sc)) = (&target_canonical, path.canonicalize()) {
                if tc == &sc {
                    continue;
                }
            } else if path == target_file_path {
                continue;
            }

            let file = match fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let reader = BufReader::new(file);
            let mut mentions = Vec::new();

            for (line_idx, line_res) in reader.lines().enumerate() {
                let line = match line_res {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                let line_lower = line.to_lowercase();

                let mut matched = false;
                let mut anchor = None;

                // Match `[[stem` or `[[stem|` or `[[stem#`
                let wikilink_pattern = format!("[[{target_stem_lower}");
                if let Some(pos) = line_lower.find(&wikilink_pattern) {
                    let after_pos = pos + wikilink_pattern.len();
                    if after_pos >= line_lower.len()
                        || line.as_bytes()[after_pos] == b']'
                        || line.as_bytes()[after_pos] == b'|'
                        || line.as_bytes()[after_pos] == b'#'
                    {
                        matched = true;
                        if after_pos < line_lower.len() && line.as_bytes()[after_pos] == b'#' {
                            if let Some(end) = line[after_pos + 1..].find(|c| c == ']' || c == '|') {
                                anchor = Some(line[after_pos + 1..after_pos + 1 + end].trim().to_string());
                            }
                        }
                    }
                }

                // Match `(filename)` or `(stem.md)` or `(/filename)`
                if !matched {
                    if line_lower.contains(&format!("({target_filename_lower})"))
                        || line_lower.contains(&format!("/{target_filename_lower})"))
                    {
                        matched = true;
                    }
                }

                if matched {
                    let trimmed = line.trim();
                    let snippet = if trimmed.chars().count() > 120 {
                        let mut s: String = trimmed.chars().take(117).collect();
                        s.push_str("...");
                        s
                    } else {
                        trimmed.to_string()
                    };

                    mentions.push(BacklinkMention {
                        line_number: line_idx + 1,
                        context_snippet: snippet,
                        target_anchor: anchor,
                    });
                }
            }

            if !mentions.is_empty() {
                let source_title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.clone());

                groups.push(BacklinkGroup {
                    source_path: path,
                    source_title,
                    mentions,
                });
            }
        }
    }

    groups.sort_by(|a, b| a.source_title.cmp(&b.source_title));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_outgoing_links() {
        let md = r#"
# Notes
Here is a reference to [[Architecture]] and an aliased [[Protocol|Our Protocol]].
Here is an anchor [[Guidelines#Coding Standards]] and a block [[API#^block123]].
Also check [Splitype Docs](https://splitype.dev) and local [Specs](specs.md#overview).
"#;
        let links = extract_outgoing_links(md);
        assert_eq!(links.len(), 6);

        assert_eq!(links[0].display_text, "Architecture");
        assert_eq!(links[0].target, "wikilink:Architecture");
        assert!(!links[0].is_external);

        assert_eq!(links[1].display_text, "Our Protocol");
        assert_eq!(links[1].target, "wikilink:Protocol");

        assert_eq!(links[2].display_text, "Guidelines > Coding Standards");
        assert_eq!(links[2].anchor.as_deref(), Some("Coding Standards"));

        assert_eq!(links[3].display_text, "API > ^block123");
        assert_eq!(links[3].anchor.as_deref(), Some("^block123"));

        assert_eq!(links[4].display_text, "Splitype Docs");
        assert_eq!(links[4].target, "https://splitype.dev");
        assert!(links[4].is_external);

        assert_eq!(links[5].display_text, "Specs");
        assert_eq!(links[5].target, "specs.md");
        assert!(!links[5].is_external);
        assert_eq!(links[5].anchor.as_deref(), Some("overview"));
    }
}
