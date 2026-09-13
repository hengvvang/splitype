//! Jump navigation history stack.
//!
//! Tracks editor jump points across documents and scroll offsets for
//! browser-style Back/Forward navigation (Alt+Left / Alt+Right).

use std::path::PathBuf;
use editor_contracts::DocumentId;

/// A single location in the jump navigation history.
#[derive(Clone, Debug, PartialEq)]
pub struct JumpPoint {
    pub document_id: DocumentId,
    pub file_path: Option<PathBuf>,
    pub scroll_y: f32,
}

/// Navigation history stack tracking back and forward jump locations.
#[derive(Clone, Debug, Default)]
pub struct EditorJumpStack {
    back_stack: Vec<JumpPoint>,
    forward_stack: Vec<JumpPoint>,
}

impl EditorJumpStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a new jump point. Clears forward history.
    pub fn push(&mut self, point: JumpPoint) {
        if let Some(top) = self.back_stack.last() {
            if top.document_id == point.document_id
                && top.file_path == point.file_path
                && (top.scroll_y - point.scroll_y).abs() < 20.0
            {
                return;
            }
        }
        self.back_stack.push(point);
        self.forward_stack.clear();
        if self.back_stack.len() > 50 {
            self.back_stack.remove(0);
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// Pops the previous jump point from the back stack, saving `current` onto forward stack.
    pub fn pop_back(&mut self, current: JumpPoint) -> Option<JumpPoint> {
        let prev = self.back_stack.pop()?;
        self.forward_stack.push(current);
        Some(prev)
    }

    /// Pops the next jump point from the forward stack, saving `current` onto back stack.
    pub fn pop_forward(&mut self, current: JumpPoint) -> Option<JumpPoint> {
        let next = self.forward_stack.pop()?;
        self.back_stack.push(current);
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_stack_navigation() {
        let mut stack = EditorJumpStack::new();
        assert!(!stack.can_go_back());
        assert!(!stack.can_go_forward());

        let doc1 = DocumentId::new();
        let doc2 = DocumentId::new();
        let doc3 = DocumentId::new();

        let p1 = JumpPoint {
            document_id: doc1,
            file_path: Some(PathBuf::from("a.md")),
            scroll_y: 0.0,
        };
        let p2 = JumpPoint {
            document_id: doc2,
            file_path: Some(PathBuf::from("b.md")),
            scroll_y: 100.0,
        };
        let p3 = JumpPoint {
            document_id: doc3,
            file_path: Some(PathBuf::from("c.md")),
            scroll_y: 200.0,
        };

        stack.push(p1.clone());
        // Deduplicate close scroll_y on same doc
        stack.push(JumpPoint {
            document_id: doc1,
            file_path: Some(PathBuf::from("a.md")),
            scroll_y: 5.0,
        });
        assert_eq!(stack.back_stack.len(), 1);

        stack.push(p2.clone());
        assert_eq!(stack.back_stack.len(), 2);
        assert!(stack.can_go_back());
        assert!(!stack.can_go_forward());

        // Now we navigate back from p3 to p2
        let back_to = stack.pop_back(p3.clone()).expect("should have back point");
        assert_eq!(back_to, p2);
        assert!(stack.can_go_back());
        assert!(stack.can_go_forward());

        // Navigate forward returns to p3
        let fwd_to = stack.pop_forward(p2.clone()).expect("should have forward point");
        assert_eq!(fwd_to, p3);
        assert!(!stack.can_go_forward());

        // Pushing new point clears forward history
        stack.push(p1.clone());
        assert!(!stack.can_go_forward());
    }
}

