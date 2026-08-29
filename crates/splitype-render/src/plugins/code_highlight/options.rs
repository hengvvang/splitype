//! Code language registry — display names, aliases, and picker options.
//!
//! This is the editor-facing language metadata (labels, values, aliases).
//! Highlighting capability itself lives in highlight.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeLanguageOption {
    pub label: &'static str,
    pub value: &'static str,
    pub aliases: &'static [&'static str],
}

macro_rules! language {
    ($label:literal, $value:literal $(, $alias:literal)*) => {
        CodeLanguageOption {
            label: $label,
            value: $value,
            aliases: &[$($alias),*],
        }
    };
}

pub const CODE_LANGUAGE_OPTIONS: &[CodeLanguageOption] = &[
    language!("ABAP", "abap"),
    language!("Agda", "agda"),
    language!("Arduino", "arduino", "ino"),
    language!("ASCII Art", "ascii", "text art"),
    language!("Assembly", "asm", "assembly"),
    language!("Bash", "bash", "sh", "shell", "zsh"),
    language!("BASIC", "basic"),
    language!("BNF", "bnf"),
    language!("C", "c"),
    language!("C#", "csharp", "cs", "c#"),
    language!("C++", "cpp", "cxx", "cc"),
    language!("Clojure", "clojure", "clj"),
    language!("CoffeeScript", "coffeescript", "coffee"),
    language!("CSS", "css"),
    language!("Dart", "dart"),
    language!("Dhall", "dhall"),
    language!("Diff", "diff", "patch"),
    language!("Dockerfile", "dockerfile", "docker"),
    language!("Elixir", "elixir", "ex"),
    language!("Elm", "elm"),
    language!("Erlang", "erlang", "erl"),
    language!("F#", "fsharp", "fs", "f#"),
    language!("Fortran", "fortran"),
    language!("Go", "go", "golang"),
    language!("GraphQL", "graphql", "gql"),
    language!("Groovy", "groovy"),
    language!("Haskell", "haskell", "hs"),
    language!("HTML", "html"),
    language!("Java", "java"),
    language!("JavaScript", "javascript", "js"),
    language!("JSON", "json"),
    language!("JSX", "jsx"),
    language!("Julia", "julia", "jl"),
    language!("Kotlin", "kotlin", "kt"),
    language!("LaTeX", "latex", "tex", "math"),
    language!("Lua", "lua"),
    language!("Markdown", "markdown", "md"),
    language!("Mermaid", "mermaid"),
    language!("Nim", "nim"),
    language!("Nix", "nix"),
    language!("Objective-C", "objective-c", "objc"),
    language!("OCaml", "ocaml", "ml"),
    language!("Perl", "perl", "pl"),
    language!("PHP", "php"),
    language!("Plain Text", "text", "plain", "txt"),
    language!("PowerShell", "powershell", "ps1"),
    language!("Python", "python", "py"),
    language!("R", "r"),
    language!("Ruby", "ruby", "rb"),
    language!("Rust", "rust", "rs"),
    language!("Scala", "scala"),
    language!("SQL", "sql"),
    language!("Swift", "swift"),
    language!("TOML", "toml"),
    language!("TSX", "tsx"),
    language!("TypeScript", "typescript", "ts"),
    language!("Vue", "vue"),
    language!("XML", "xml"),
    language!("YAML", "yaml", "yml"),
    language!("Zig", "zig"),
];

pub fn code_language_options_matching(query: &str) -> Vec<&'static CodeLanguageOption> {
    let query = query.trim().to_lowercase();
    CODE_LANGUAGE_OPTIONS
        .iter()
        .filter(|option| {
            query.is_empty()
                || option.label.to_lowercase().contains(&query)
                || option.value.contains(&query)
                || option.aliases.iter().any(|alias| alias.contains(&query))
        })
        .collect()
}

pub fn code_language_display_name(value: &str) -> &str {
    CODE_LANGUAGE_OPTIONS
        .iter()
        .find(|option| {
            option.value.eq_ignore_ascii_case(value)
                || option
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(value))
        })
        .map(|option| option.label)
        .unwrap_or(value)
}
