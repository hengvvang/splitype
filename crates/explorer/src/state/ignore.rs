//! Git ignore stack — mirrors Zed's `crates/worktree/src/ignore.rs`.
//!
//! Evaluates `.gitignore` files hierarchically down the worktree directory tree.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Clone, Debug)]
pub struct IgnoreStack {
    pub repo_root: Option<PathBuf>,
    pub top: Arc<IgnoreStackEntry>,
}

#[derive(Debug)]
pub enum IgnoreStackEntry {
    None,
    Layer {
        base_dir: PathBuf,
        ignore: Arc<Gitignore>,
        parent: Arc<IgnoreStackEntry>,
    },
}

impl Default for IgnoreStack {
    fn default() -> Self {
        Self::none()
    }
}

impl IgnoreStack {
    pub fn none() -> Self {
        Self {
            repo_root: None,
            top: Arc::new(IgnoreStackEntry::None),
        }
    }

    pub fn with_root(root: &Path) -> Self {
        let mut stack = Self {
            repo_root: Some(root.to_path_buf()),
            top: Arc::new(IgnoreStackEntry::None),
        };
        stack.push_dir_gitignore(root);
        stack
    }

    /// If `dir` contains a `.gitignore` file, parse it and push a new layer onto the stack.
    pub fn push_dir_gitignore(&mut self, dir: &Path) {
        let gitignore_path = dir.join(".gitignore");
        if gitignore_path.is_file() {
            let mut builder = GitignoreBuilder::new(dir);
            if let Some(err) = builder.add(&gitignore_path) {
                tracing::debug!(path = %gitignore_path.display(), error = %err, "failed to parse .gitignore");
            }
            if let Ok(ignore) = builder.build() {
                self.top = Arc::new(IgnoreStackEntry::Layer {
                    base_dir: dir.to_path_buf(),
                    ignore: Arc::new(ignore),
                    parent: self.top.clone(),
                });
            }
        }
    }

    /// Add custom gitignore rules directly (useful for tests or global config).
    pub fn push_custom_rules(&mut self, base_dir: &Path, rules: &[&str]) {
        let mut builder = GitignoreBuilder::new(base_dir);
        for rule in rules {
            let _ = builder.add_line(None, rule);
        }
        if let Ok(ignore) = builder.build() {
            self.top = Arc::new(IgnoreStackEntry::Layer {
                base_dir: base_dir.to_path_buf(),
                ignore: Arc::new(ignore),
                parent: self.top.clone(),
            });
        }
    }

    /// Evaluates whether `path` is ignored under this stack.
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        // Automatically ignore .git directory itself
        if is_dir && path.file_name() == Some(std::ffi::OsStr::new(".git")) {
            return true;
        }

        let mut current = self.top.as_ref();
        while let IgnoreStackEntry::Layer {
            base_dir,
            ignore,
            parent,
        } = current
        {
            if path.starts_with(base_dir) {
                match ignore.matched_path_or_any_parents(path, is_dir) {
                    ignore::Match::Ignore(_) => return true,
                    ignore::Match::Whitelist(_) => return false,
                    ignore::Match::None => {}
                }
            }
            current = parent.as_ref();
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_stack_basic_and_whitelist() {
        let temp_dir = std::env::temp_dir().join(format!(
            "splitype_test_ignore_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut stack = IgnoreStack::none();
        stack.push_custom_rules(
            &temp_dir,
            &["target/", "*.log", "!keep.log", "node_modules/"],
        );

        let target_dir = temp_dir.join("target");
        let node_modules = temp_dir.join("node_modules");
        let app_log = temp_dir.join("app.log");
        let keep_log = temp_dir.join("keep.log");
        let src_rs = temp_dir.join("src").join("main.rs");

        assert!(stack.is_ignored(&target_dir, true));
        assert!(stack.is_ignored(&node_modules, true));
        assert!(stack.is_ignored(&app_log, false));
        assert!(!stack.is_ignored(&keep_log, false), "keep.log was whitelisted with !");
        assert!(!stack.is_ignored(&src_rs, false));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
