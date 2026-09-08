//! Syntax-aware symbol and outline extraction powered by Tree-Sitter ASTs.

use editor_contracts::{OutlineNode, OutlineNodeKind};
use tree_sitter::{Node, Parser};

use crate::highlight::language_config;
use crate::language::CodeLanguageKey;

/// Extracts outline symbol nodes from the given source text using the
/// Tree-Sitter grammar for the given language.
///
/// Returns an empty list if the language has no Tree-Sitter grammar configured.
pub fn extract_symbols(key: CodeLanguageKey, text: &str) -> Vec<OutlineNode> {
    let Some(config) = language_config(key) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if parser.set_language(&(config.grammar)()).is_err() {
        return Vec::new();
    }

    let Some(tree) = parser.parse(text.as_bytes(), None) else {
        return Vec::new();
    };

    let mut symbols = Vec::new();
    let root = tree.root_node();
    walk_node(&root, text, key, 1, false, &mut symbols);
    symbols
}

/// Recursively walks AST nodes and collects symbol outline nodes.
fn walk_node(
    node: &Node,
    text: &str,
    key: CodeLanguageKey,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match key {
        CodeLanguageKey::Rust => {
            walk_rust_node(node, text, level, in_container, out);
        }
        CodeLanguageKey::Python => {
            walk_python_node(node, text, level, in_container, out);
        }
        CodeLanguageKey::JavaScript
        | CodeLanguageKey::JavaScriptJsx
        | CodeLanguageKey::TypeScript
        | CodeLanguageKey::TypeScriptTsx => {
            walk_js_ts_node(node, text, level, in_container, out);
        }
        CodeLanguageKey::Go => {
            walk_go_node(node, text, level, in_container, out);
        }
        CodeLanguageKey::C | CodeLanguageKey::Cpp => {
            walk_c_cpp_node(node, text, level, in_container, out);
        }
        CodeLanguageKey::Toml => {
            walk_toml_node(node, text, level, out);
        }
        CodeLanguageKey::Json => {
            walk_json_node(node, text, level, out);
        }
        _ => {
            walk_generic_node(node, text, level, in_container, out);
        }
    }
}

// ── Rust ─────────────────────────────────────────────────────────────────

fn walk_rust_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = find_name_node(node, text) {
                let kind = if in_container {
                    OutlineNodeKind::Method
                } else {
                    OutlineNodeKind::Function
                };
                let sym_level = if in_container { 2 } else { 1 };
                push_symbol(out, node, name, sym_level, kind);
            }
        }
        "struct_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Struct);
            }
        }
        "enum_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Enum);
            }
        }
        "trait_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Trait);
            }
            walk_children(node, text, CodeLanguageKey::Rust, level.saturating_add(1), true, out);
        }
        "impl_item" => {
            let label = extract_rust_impl_title(node, text);
            push_symbol(out, node, &label, 1, OutlineNodeKind::Section);
            walk_children(node, text, CodeLanguageKey::Rust, level.saturating_add(1), true, out);
        }
        "mod_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Module);
            }
            walk_children(node, text, CodeLanguageKey::Rust, level.saturating_add(1), in_container, out);
        }
        "const_item" | "static_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, level, OutlineNodeKind::Constant);
            }
        }
        "type_item" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, level, OutlineNodeKind::Struct);
            }
        }
        "macro_definition" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, level, OutlineNodeKind::Section);
            }
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::Rust, level, in_container, out);
        }
    }
}

fn extract_rust_impl_title(node: &Node, text: &str) -> String {
    let raw = node_slice(node, text);
    let head = raw.split('{').next().unwrap_or(raw);
    let first_line = head.lines().next().unwrap_or(head).trim();
    if first_line.is_empty() {
        "impl".to_string()
    } else {
        first_line.to_string()
    }
}

// ── Python ───────────────────────────────────────────────────────────────

fn walk_python_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match node.kind() {
        "class_definition" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Class);
            }
            walk_children(node, text, CodeLanguageKey::Python, level.saturating_add(1), true, out);
        }
        "function_definition" => {
            if let Some(name) = find_name_node(node, text) {
                let kind = if in_container {
                    OutlineNodeKind::Method
                } else {
                    OutlineNodeKind::Function
                };
                let sym_level = if in_container { 2 } else { 1 };
                push_symbol(out, node, name, sym_level, kind);
            }
            walk_children(node, text, CodeLanguageKey::Python, level.saturating_add(1), in_container, out);
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::Python, level, in_container, out);
        }
    }
}

// ── JavaScript / TypeScript ──────────────────────────────────────────────

fn walk_js_ts_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Class);
            }
            walk_children(node, text, CodeLanguageKey::TypeScript, level.saturating_add(1), true, out);
        }
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                let kind = if in_container {
                    OutlineNodeKind::Method
                } else {
                    OutlineNodeKind::Function
                };
                let sym_level = if in_container { 2 } else { 1 };
                push_symbol(out, node, name, sym_level, kind);
            }
        }
        "method_definition" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 2, OutlineNodeKind::Method);
            }
        }
        "interface_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Trait);
            }
        }
        "type_alias_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Struct);
            }
        }
        "enum_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Enum);
            }
        }
        "variable_declarator" => {
            if let Some(value) = node.child_by_field_name("value") {
                if matches!(value.kind(), "arrow_function" | "function_expression") {
                    if let Some(name) = find_name_node(node, text) {
                        push_symbol(out, node, name, level, OutlineNodeKind::Function);
                    }
                }
            }
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::TypeScript, level, in_container, out);
        }
    }
}

// ── Go ───────────────────────────────────────────────────────────────────

fn walk_go_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Function);
            }
        }
        "method_declaration" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 2, OutlineNodeKind::Method);
            }
        }
        "type_spec" => {
            if let Some(name) = find_name_node(node, text) {
                let kind = if let Some(type_node) = node.child_by_field_name("type") {
                    match type_node.kind() {
                        "struct_type" => OutlineNodeKind::Struct,
                        "interface_type" => OutlineNodeKind::Trait,
                        _ => OutlineNodeKind::Struct,
                    }
                } else {
                    OutlineNodeKind::Struct
                };
                push_symbol(out, node, name, 1, kind);
            }
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::Go, level, in_container, out);
        }
    }
}

// ── C / C++ ──────────────────────────────────────────────────────────────

fn walk_c_cpp_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = find_c_function_name(node, text) {
                let kind = if in_container {
                    OutlineNodeKind::Method
                } else {
                    OutlineNodeKind::Function
                };
                let sym_level = if in_container { 2 } else { 1 };
                push_symbol(out, node, name, sym_level, kind);
            }
            walk_children(node, text, CodeLanguageKey::Cpp, level.saturating_add(1), in_container, out);
        }
        "struct_specifier" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Struct);
            }
            walk_children(node, text, CodeLanguageKey::Cpp, level.saturating_add(1), true, out);
        }
        "class_specifier" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Class);
            }
            walk_children(node, text, CodeLanguageKey::Cpp, level.saturating_add(1), true, out);
        }
        "enum_specifier" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Enum);
            }
        }
        "namespace_definition" => {
            if let Some(name) = find_name_node(node, text) {
                push_symbol(out, node, name, 1, OutlineNodeKind::Module);
            }
            walk_children(node, text, CodeLanguageKey::Cpp, level.saturating_add(1), in_container, out);
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::Cpp, level, in_container, out);
        }
    }
}

fn find_c_function_name<'a>(node: &Node<'a>, text: &'a str) -> Option<&'a str> {
    let declarator = node.child_by_field_name("declarator")?;
    let mut curr = declarator;
    loop {
        if curr.kind() == "identifier" || curr.kind() == "field_identifier" {
            return Some(node_slice(&curr, text));
        }
        if let Some(child) = curr.child_by_field_name("declarator") {
            curr = child;
        } else if let Some(first) = curr.named_child(0) {
            curr = first;
        } else {
            break;
        }
    }
    None
}

// ── TOML ─────────────────────────────────────────────────────────────────

fn walk_toml_node(node: &Node, text: &str, level: u8, out: &mut Vec<OutlineNode>) {
    match node.kind() {
        "table" | "table_array_element" => {
            let title = node_slice(node, text);
            let first_line = title.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                push_symbol(out, node, first_line, 1, OutlineNodeKind::Section);
            }
        }
        _ => {
            walk_children(node, text, CodeLanguageKey::Toml, level, false, out);
        }
    }
}

// ── JSON ─────────────────────────────────────────────────────────────────

fn walk_json_node(node: &Node, text: &str, level: u8, out: &mut Vec<OutlineNode>) {
    match node.kind() {
        "pair" => {
            if let Some(key_node) = node.child_by_field_name("key") {
                let key_text = node_slice(&key_node, text).trim_matches('"');
                if !key_text.is_empty() {
                    push_symbol(out, node, key_text, level, OutlineNodeKind::Section);
                }
            }
        }
        "document" | "object" => {
            let next_level = if node.kind() == "object" { level.saturating_add(1) } else { 1 };
            if next_level <= 2 {
                walk_children(node, text, CodeLanguageKey::Json, next_level, false, out);
            }
        }
        _ => {}
    }
}

// ── Generic Fallback ─────────────────────────────────────────────────────

fn walk_generic_node(
    node: &Node,
    text: &str,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    let kind_str = node.kind().to_ascii_lowercase();

    let matched_kind = if kind_str.contains("func") || kind_str.contains("fn") {
        Some(if in_container {
            OutlineNodeKind::Method
        } else {
            OutlineNodeKind::Function
        })
    } else if kind_str.contains("method") {
        Some(OutlineNodeKind::Method)
    } else if kind_str.contains("class") {
        Some(OutlineNodeKind::Class)
    } else if kind_str.contains("struct") {
        Some(OutlineNodeKind::Struct)
    } else if kind_str.contains("enum") {
        Some(OutlineNodeKind::Enum)
    } else if kind_str.contains("trait") || kind_str.contains("interface") {
        Some(OutlineNodeKind::Trait)
    } else if kind_str.contains("module") || kind_str.contains("namespace") {
        Some(OutlineNodeKind::Module)
    } else {
        None
    };

    if let Some(sym_kind) = matched_kind {
        if let Some(name) = find_name_node(node, text) {
            let sym_level = if in_container { 2 } else { 1 };
            push_symbol(out, node, name, sym_level, sym_kind);
        }
    }

    let is_new_container = kind_str.contains("class")
        || kind_str.contains("struct")
        || kind_str.contains("impl");

    walk_children(
        node,
        text,
        CodeLanguageKey::PlainText,
        level.saturating_add(1),
        in_container || is_new_container,
        out,
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn walk_children(
    node: &Node,
    text: &str,
    key: CodeLanguageKey,
    level: u8,
    in_container: bool,
    out: &mut Vec<OutlineNode>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(&child, text, key, level, in_container, out);
    }
}

fn push_symbol(
    out: &mut Vec<OutlineNode>,
    node: &Node,
    label: &str,
    level: u8,
    kind: OutlineNodeKind,
) {
    let line_idx = node.start_position().row;
    let byte_offset = node.start_byte();
    let id = format!("outline:sym:{line_idx}:{byte_offset}");
    out.push(OutlineNode::symbol(
        id,
        label.trim().to_string(),
        level,
        line_idx,
        kind,
    ));
}

fn find_name_node<'a>(node: &Node<'a>, text: &'a str) -> Option<&'a str> {
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = node_slice(&name_node, text);
        if !name.is_empty() {
            return Some(name);
        }
    }

    // Try scanning children for common identifier types
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier" => {
                let name = node_slice(&child, text);
                if !name.is_empty() {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

#[inline]
fn node_slice<'a>(node: &Node<'a>, text: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    if start <= end && end <= text.len() {
        &text[start..end]
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_symbols() {
        let code = r#"
mod parser;

pub struct Editor {
    pub text: String,
}

pub enum State {
    Idle,
    Running,
}

pub trait Runnable {
    fn run(&self);
}

impl Editor {
    pub fn new() -> Self {
        Self { text: String::new() }
    }

    fn reset(&mut self) {}
}

fn helper() {}
"#;

        let symbols = extract_symbols(CodeLanguageKey::Rust, code);
        assert!(!symbols.is_empty());

        let names: Vec<(&str, OutlineNodeKind, u8)> = symbols
            .iter()
            .map(|s| (s.label.as_str(), s.kind, s.level))
            .collect();

        assert!(names.iter().any(|(name, kind, _)| *name == "parser" && *kind == OutlineNodeKind::Module));
        assert!(names.iter().any(|(name, kind, _)| *name == "Editor" && *kind == OutlineNodeKind::Struct));
        assert!(names.iter().any(|(name, kind, _)| *name == "State" && *kind == OutlineNodeKind::Enum));
        assert!(names.iter().any(|(name, kind, _)| *name == "Runnable" && *kind == OutlineNodeKind::Trait));
        assert!(names.iter().any(|(name, kind, _)| name.starts_with("impl") && *kind == OutlineNodeKind::Section));
        assert!(names.iter().any(|(name, kind, lvl)| *name == "new" && *kind == OutlineNodeKind::Method && *lvl == 2));
        assert!(names.iter().any(|(name, kind, lvl)| *name == "helper" && *kind == OutlineNodeKind::Function && *lvl == 1));
    }

    #[test]
    fn extracts_python_symbols() {
        let py = r#"
class Controller:
    def __init__(self):
        pass

    def run(self):
        pass

def standalone():
    pass
"#;

        let symbols = extract_symbols(CodeLanguageKey::Python, py);
        assert_eq!(symbols.len(), 4);
        assert_eq!(symbols[0].label, "Controller");
        assert_eq!(symbols[0].kind, OutlineNodeKind::Class);
        assert_eq!(symbols[1].label, "__init__");
        assert_eq!(symbols[1].kind, OutlineNodeKind::Method);
        assert_eq!(symbols[1].level, 2);
        assert_eq!(symbols[2].label, "run");
        assert_eq!(symbols[2].kind, OutlineNodeKind::Method);
        assert_eq!(symbols[2].level, 2);
        assert_eq!(symbols[3].label, "standalone");
        assert_eq!(symbols[3].kind, OutlineNodeKind::Function);
        assert_eq!(symbols[3].level, 1);
    }
}
