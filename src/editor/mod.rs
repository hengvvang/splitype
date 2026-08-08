//! Document editing runtime — controller, document tree, block entities,
//! undo/selection/layout-geometry logic, and block-event protocol.

pub mod actions;
pub mod block_protocol;
pub mod commands;
pub mod controller;
pub mod editing;
pub mod file;
pub mod geometry;
pub mod keybindings;
pub mod panels;
pub mod render;
pub mod tree;

#[cfg(test)]
mod tests;
