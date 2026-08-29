#[cfg(test)]
mod tests {
    use crate::root::SplitterRoot;
    use crate::tree::{NodeId, SplitAxis};
    use gpui::{px, size};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum MockKind {
        Editor,
        Preview,
        Outline,
    }

    #[test]
    fn test_single_leaf_split_tree_initialization() {
        let root = SplitterRoot::single_leaf(1, MockKind::Editor);
        assert_eq!(root.tree.count_leaves(), 1);
        assert_eq!(root.tree.first_leaf_id(), Some(1));
    }

    #[test]
    fn test_split_leaf_horizontal_and_vertical() {
        let mut root = SplitterRoot::single_leaf(1, MockKind::Editor);
        let first_leaf_id: NodeId = 1;

        // Split horizontally
        let new_leaf_id = root
            .split_leaf(first_leaf_id, SplitAxis::Horizontal, 0.5)
            .expect("split should succeed");

        assert_eq!(root.tree.count_leaves(), 2);
        assert_ne!(first_leaf_id, new_leaf_id);

        // Split the new leaf vertically
        let third_leaf_id = root
            .split_leaf(new_leaf_id, SplitAxis::Vertical, 0.5)
            .expect("split vertical should succeed");

        assert_eq!(root.tree.count_leaves(), 3);
        assert_ne!(new_leaf_id, third_leaf_id);
    }

    #[test]
    fn test_leaf_rects_fill_viewport_bounds() {
        let mut root = SplitterRoot::single_leaf(1, MockKind::Editor);
        root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();

        let rects = root.leaf_rects(size(px(1000.0), px(600.0)));
        assert_eq!(rects.len(), 2);

        let total_width: f32 = rects.iter().map(|r| r.width).sum();
        assert!((total_width - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_close_leaf_collapses_tree() {
        let mut root = SplitterRoot::single_leaf(1, MockKind::Editor);
        let second_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();

        assert_eq!(root.tree.count_leaves(), 2);
        root.close_leaf(second_id);
        assert_eq!(root.tree.count_leaves(), 1);
        assert_eq!(root.tree.first_leaf_id(), Some(1));
    }
}
