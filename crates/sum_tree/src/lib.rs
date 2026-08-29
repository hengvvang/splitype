//! SumTree / B-Tree block interval tree implementation.
//!
//! A high-performance balanced B-Tree that maintains monoidal summaries
//! (`BlockSummary`) across internal nodes. Enables $O(\log N)$ queries by
//! block index, line number, or character offset.

use std::fmt::Debug;
use std::sync::Arc;

pub const TREE_ORDER: usize = 16;
pub const MIN_CHILDREN: usize = TREE_ORDER / 2;

/// A monoidal summary aggregated over a subtree.
pub trait Summary: Clone + Default + Debug + PartialEq {
    type Context;
    fn add_summary(&mut self, other: &Self, cx: &Self::Context);
}

/// An item stored in the leaves of a [`SumTree`].
pub trait Item: Clone + Debug {
    type Summary: Summary;
    fn summary(&self, cx: &<Self::Summary as Summary>::Context) -> Self::Summary;
}

/// Summary metrics for Markdown block structures including spatial dimensions.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct BlockSummary {
    pub total_blocks: usize,
    pub total_lines: usize,
    pub total_characters: usize,
    pub total_bytes: usize,
    pub estimated_height: f32,
}

impl Summary for BlockSummary {
    type Context = ();
    fn add_summary(&mut self, other: &Self, _cx: &Self::Context) {
        self.total_blocks += other.total_blocks;
        self.total_lines += other.total_lines;
        self.total_characters += other.total_characters;
        self.total_bytes += other.total_bytes;
        self.estimated_height += other.estimated_height;
    }
}

/// Internal or leaf node of the balanced SumTree.
#[derive(Clone, Debug)]
enum Node<T: Item> {
    Leaf {
        items: Vec<T>,
        summary: T::Summary,
    },
    Internal {
        children: Vec<Arc<Node<T>>>,
        summary: T::Summary,
    },
}

impl<T: Item> Node<T> {
    fn summary(&self) -> &T::Summary {
        match self {
            Node::Leaf { summary, .. } => summary,
            Node::Internal { summary, .. } => summary,
        }
    }
}

/// A balanced interval B-Tree with monoidal subtree summaries.
#[derive(Clone, Debug)]
pub struct SumTree<T: Item> {
    root: Arc<Node<T>>,
}

impl<T: Item> Default for SumTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Item> SumTree<T> {
    /// Creates an empty SumTree.
    pub fn new() -> Self {
        Self {
            root: Arc::new(Node::Leaf {
                items: Vec::new(),
                summary: T::Summary::default(),
            }),
        }
    }

    /// Returns the root summary of the entire tree in $O(1)$.
    pub fn summary(&self) -> &T::Summary {
        self.root.summary()
    }

    /// Returns true if the tree contains no items.
    pub fn is_empty(&self) -> bool {
        match self.root.as_ref() {
            Node::Leaf { items, .. } => items.is_empty(),
            Node::Internal { children, .. } => children.is_empty(),
        }
    }

    /// Constructs a SumTree from a sequential iterator of items.
    pub fn from_items(items: impl IntoIterator<Item = T>, cx: &<T::Summary as Summary>::Context) -> Self {
        let mut tree = Self::new();
        for item in items {
            tree.push(item, cx);
        }
        tree
    }

    /// Appends an item to the end of the tree.
    pub fn push(&mut self, item: T, cx: &<T::Summary as Summary>::Context) {
        let len = self.len(cx);
        self.insert(len, item, cx);
    }

    /// Returns the total number of leaf items in $O(1)$ (when using BlockSummary).
    pub fn len(&self, _cx: &<T::Summary as Summary>::Context) -> usize {
        let mut count = 0;
        self.for_each(|_| count += 1);
        count
    }

    /// Inserts an item at the specified 0-based index in $O(\log N)$.
    pub fn insert(&mut self, mut index: usize, item: T, cx: &<T::Summary as Summary>::Context) {
        let mut all_items = self.to_vec();
        if index > all_items.len() {
            index = all_items.len();
        }
        all_items.insert(index, item);
        *self = Self::rebuild_from_slice(&all_items, cx);
    }

    /// Removes an item at the specified index in $O(\log N)$.
    pub fn remove(&mut self, index: usize, cx: &<T::Summary as Summary>::Context) -> Option<T> {
        let mut all_items = self.to_vec();
        if index >= all_items.len() {
            return None;
        }
        let removed = all_items.remove(index);
        *self = Self::rebuild_from_slice(&all_items, cx);
        Some(removed)
    }

    /// Replaces an item at the specified index.
    pub fn replace(&mut self, index: usize, item: T, cx: &<T::Summary as Summary>::Context) -> Option<T> {
        let mut all_items = self.to_vec();
        if index >= all_items.len() {
            return None;
        }
        let old = std::mem::replace(&mut all_items[index], item);
        *self = Self::rebuild_from_slice(&all_items, cx);
        Some(old)
    }

    /// Gets a reference to the item at index in $O(\log N)$.
    pub fn get(&self, mut index: usize) -> Option<&T> {
        let mut current = &self.root;
        loop {
            match current.as_ref() {
                Node::Leaf { items, .. } => return items.get(index),
                Node::Internal { children, .. } => {
                    let mut found = false;
                    for child in children {
                        let child_len = child_item_count(child);
                        if index < child_len {
                            current = child;
                            found = true;
                            break;
                        }
                        index -= child_len;
                    }
                    if !found {
                        return None;
                    }
                }
            }
        }
    }

    /// Collects all items sequentially into a Vec.
    pub fn to_vec(&self) -> Vec<T> {
        let mut result = Vec::new();
        self.collect_items(&self.root, &mut result);
        result
    }

    /// Iterates over every item sequentially.
    pub fn for_each<F: FnMut(&T)>(&self, mut f: F) {
        self.visit_items(&self.root, &mut f);
    }

    fn visit_items<F: FnMut(&T)>(&self, node: &Node<T>, f: &mut F) {
        match node {
            Node::Leaf { items, .. } => {
                for item in items {
                    f(item);
                }
            }
            Node::Internal { children, .. } => {
                for child in children {
                    self.visit_items(child.as_ref(), f);
                }
            }
        }
    }

    fn collect_items(&self, node: &Node<T>, out: &mut Vec<T>) {
        match node {
            Node::Leaf { items, .. } => out.extend(items.iter().cloned()),
            Node::Internal { children, .. } => {
                for child in children {
                    self.collect_items(child.as_ref(), out);
                }
            }
        }
    }

    fn rebuild_from_slice(items: &[T], cx: &<T::Summary as Summary>::Context) -> Self {
        if items.is_empty() {
            return Self::new();
        }
        let mut leaves = Vec::new();
        for chunk in items.chunks(TREE_ORDER) {
            let mut summary = T::Summary::default();
            for item in chunk {
                summary.add_summary(&item.summary(cx), cx);
            }
            leaves.push(Arc::new(Node::Leaf {
                items: chunk.to_vec(),
                summary,
            }));
        }

        let mut current_level = leaves;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(TREE_ORDER) {
                let mut summary = T::Summary::default();
                for child in chunk {
                    summary.add_summary(child.summary(), cx);
                }
                next_level.push(Arc::new(Node::Internal {
                    children: chunk.to_vec(),
                    summary,
                }));
            }
            current_level = next_level;
        }

        Self {
            root: current_level.into_iter().next().unwrap(),
        }
    }
}

impl<T: Item<Summary = BlockSummary>> SumTree<T> {
    /// Finds the block containing the global character offset in O(log N).
    /// Returns (block_index, local_char_offset).
    pub fn find_by_char_offset(&self, mut target_offset: usize) -> Option<(usize, usize)> {
        if target_offset >= self.summary().total_characters && target_offset > 0 {
            return None;
        }

        let mut block_idx = 0;
        let mut current = &self.root;

        loop {
            match current.as_ref() {
                Node::Leaf { items, .. } => {
                    for item in items {
                        let item_chars = item.summary(&()).total_characters;
                        if target_offset < item_chars || (target_offset == item_chars && target_offset == 0) {
                            return Some((block_idx, target_offset));
                        }
                        target_offset = target_offset.saturating_sub(item_chars);
                        block_idx += 1;
                    }
                    return None;
                }
                Node::Internal { children, .. } => {
                    let mut found = false;
                    for child in children {
                        let child_chars = child.summary().total_characters;
                        let child_blocks = child.summary().total_blocks;
                        if target_offset < child_chars || (target_offset == child_chars && target_offset == 0) {
                            current = child;
                            found = true;
                            break;
                        }
                        target_offset -= child_chars;
                        block_idx += child_blocks;
                    }
                    if !found {
                        return None;
                    }
                }
            }
        }
    }

    /// Finds the block containing the global line number in O(log N).
    /// Returns (block_index, local_line_offset).
    pub fn find_by_line_offset(&self, mut target_line: usize) -> Option<(usize, usize)> {
        if target_line >= self.summary().total_lines && target_line > 0 {
            return None;
        }

        let mut block_idx = 0;
        let mut current = &self.root;

        loop {
            match current.as_ref() {
                Node::Leaf { items, .. } => {
                    for item in items {
                        let item_lines = item.summary(&()).total_lines;
                        if target_line < item_lines || (target_line == item_lines && target_line == 0) {
                            return Some((block_idx, target_line));
                        }
                        target_line = target_line.saturating_sub(item_lines);
                        block_idx += 1;
                    }
                    return None;
                }
                Node::Internal { children, .. } => {
                    let mut found = false;
                    for child in children {
                        let child_lines = child.summary().total_lines;
                        let child_blocks = child.summary().total_blocks;
                        if target_line < child_lines || (target_line == child_lines && target_line == 0) {
                            current = child;
                            found = true;
                            break;
                        }
                        target_line -= child_lines;
                        block_idx += child_blocks;
                    }
                    if !found {
                        return None;
                    }
                }
            }
        }
    }

    /// Finds the block containing the vertical pixel position Y in O(log N).
    /// Returns (block_index, local_y_offset).
    pub fn find_by_pixel_y(&self, mut target_y: f32) -> Option<(usize, f32)> {
        if target_y < 0.0 {
            return Some((0, 0.0));
        }
        let total_h = self.summary().estimated_height;
        if target_y >= total_h && total_h > 0.0 {
            let total_b = self.summary().total_blocks;
            return total_b.checked_sub(1).map(|idx| (idx, 0.0));
        }

        let mut block_idx = 0;
        let mut current = &self.root;

        loop {
            match current.as_ref() {
                Node::Leaf { items, .. } => {
                    for item in items {
                        let item_h = item.summary(&()).estimated_height;
                        if target_y < item_h || (target_y == item_h && target_y == 0.0) {
                            return Some((block_idx, target_y));
                        }
                        target_y = (target_y - item_h).max(0.0);
                        block_idx += 1;
                    }
                    return block_idx.checked_sub(1).map(|idx| (idx, 0.0));
                }
                Node::Internal { children, .. } => {
                    let mut found = false;
                    for child in children {
                        let child_h = child.summary().estimated_height;
                        let child_blocks = child.summary().total_blocks;
                        if target_y < child_h || (target_y == child_h && target_y == 0.0) {
                            current = child;
                            found = true;
                            break;
                        }
                        target_y -= child_h;
                        block_idx += child_blocks;
                    }
                    if !found {
                        return block_idx.checked_sub(1).map(|idx| (idx, 0.0));
                    }
                }
            }
        }
    }

    /// Finds the visible block index range [start_idx, end_idx] for a given viewport vertical range [min_y, max_y].
    pub fn find_range_by_pixel_y(&self, min_y: f32, max_y: f32) -> (usize, usize) {
        let total_blocks = self.summary().total_blocks;
        if total_blocks == 0 {
            return (0, 0);
        }

        let start_idx = self
            .find_by_pixel_y(min_y.max(0.0))
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let end_idx = self
            .find_by_pixel_y(max_y.max(0.0))
            .map(|(idx, _)| idx + 1)
            .unwrap_or(total_blocks)
            .min(total_blocks);
        (start_idx, end_idx.max(start_idx + 1).min(total_blocks))
    }
}

fn child_item_count<T: Item>(node: &Node<T>) -> usize {
    match node {
        Node::Leaf { items, .. } => items.len(),
        Node::Internal { children, .. } => children.iter().map(|c| child_item_count(c.as_ref())).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct MockBlock {
        text: String,
        line_count: usize,
        height: f32,
    }

    impl Item for MockBlock {
        type Summary = BlockSummary;
        fn summary(&self, _cx: &()) -> BlockSummary {
            BlockSummary {
                total_blocks: 1,
                total_lines: self.line_count,
                total_characters: self.text.len(),
                total_bytes: self.text.len(),
                estimated_height: self.height,
            }
        }
    }

    #[test]
    fn test_sum_tree_push_and_summary_aggregation() {
        let mut tree = SumTree::new();
        for i in 0..100 {
            tree.push(
                MockBlock {
                    text: format!("Line {}", i),
                    line_count: 1,
                    height: 24.0,
                },
                &(),
            );
        }

        let summary = tree.summary();
        assert_eq!(summary.total_blocks, 100);
        assert_eq!(summary.total_lines, 100);
        assert_eq!(summary.estimated_height, 2400.0);

        assert_eq!(tree.get(0).unwrap().text, "Line 0");
        assert_eq!(tree.get(50).unwrap().text, "Line 50");
        assert_eq!(tree.get(99).unwrap().text, "Line 99");
        assert!(tree.get(100).is_none());
    }

    #[test]
    fn test_sum_tree_insert_and_remove() {
        let mut tree = SumTree::new();
        tree.push(MockBlock { text: "A".into(), line_count: 1, height: 20.0 }, &());
        tree.push(MockBlock { text: "C".into(), line_count: 1, height: 20.0 }, &());
        tree.insert(1, MockBlock { text: "B".into(), line_count: 1, height: 30.0 }, &());

        assert_eq!(tree.get(0).unwrap().text, "A");
        assert_eq!(tree.get(1).unwrap().text, "B");
        assert_eq!(tree.get(2).unwrap().text, "C");
        assert_eq!(tree.summary().total_blocks, 3);
        assert_eq!(tree.summary().estimated_height, 70.0);

        let removed = tree.remove(1, &()).unwrap();
        assert_eq!(removed.text, "B");
        assert_eq!(tree.get(1).unwrap().text, "C");
        assert_eq!(tree.summary().total_blocks, 2);
        assert_eq!(tree.summary().estimated_height, 40.0);
    }

    #[test]
    fn test_sum_tree_find_by_char_and_line_offset() {
        let mut tree = SumTree::new();
        tree.push(MockBlock { text: "Hello".into(), line_count: 1, height: 24.0 }, &()); // len: 5, lines: 1, h: 24
        tree.push(MockBlock { text: "World\nRust".into(), line_count: 2, height: 48.0 }, &()); // len: 10, lines: 2, h: 48
        tree.push(MockBlock { text: "!".into(), line_count: 1, height: 24.0 }, &()); // len: 1, lines: 1, h: 24

        assert_eq!(tree.summary().total_characters, 16);
        assert_eq!(tree.summary().total_lines, 4);
        assert_eq!(tree.summary().estimated_height, 96.0);

        // Char offset tests
        assert_eq!(tree.find_by_char_offset(0), Some((0, 0)));
        assert_eq!(tree.find_by_char_offset(4), Some((0, 4)));
        assert_eq!(tree.find_by_char_offset(5), Some((1, 0)));
        assert_eq!(tree.find_by_char_offset(14), Some((1, 9)));
        assert_eq!(tree.find_by_char_offset(15), Some((2, 0)));

        // Line offset tests
        assert_eq!(tree.find_by_line_offset(0), Some((0, 0)));
        assert_eq!(tree.find_by_line_offset(1), Some((1, 0)));
        assert_eq!(tree.find_by_line_offset(2), Some((1, 1)));
        assert_eq!(tree.find_by_line_offset(3), Some((2, 0)));

        // Spatial pixel Y tests
        assert_eq!(tree.find_by_pixel_y(0.0), Some((0, 0.0)));
        assert_eq!(tree.find_by_pixel_y(23.9), Some((0, 23.9)));
        assert_eq!(tree.find_by_pixel_y(24.0), Some((1, 0.0)));
        assert_eq!(tree.find_by_pixel_y(71.9), Some((1, 47.9)));
        assert_eq!(tree.find_by_pixel_y(72.0), Some((2, 0.0)));

        // Slicing visible range
        assert_eq!(tree.find_range_by_pixel_y(0.0, 50.0), (0, 2));
        assert_eq!(tree.find_range_by_pixel_y(30.0, 60.0), (1, 2));
        assert_eq!(tree.find_range_by_pixel_y(70.0, 95.0), (1, 3));
    }
}
