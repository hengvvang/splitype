//! Capability declarations for panel plugins.

/// What a [`super::PanelView`] (or its [`super::PanelDescriptor`]) is able
/// to do.
///
/// Hosts gate role assignment on these flags instead of comparing kind
/// strings, so any plugin can take over a built-in role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PanelCapabilities {
    /// The panel routes documents: it owns tabs of editable files and
    /// answers document lifecycle questions through [`super::DocumentPanel`].
    pub documents: bool,
    /// The panel is a sidebar companion, shown beside document panels by the
    /// default window layout.
    pub sidebar: bool,
}

impl PanelCapabilities {
    pub const NONE: Self = Self {
        documents: false,
        sidebar: false,
    };

    pub const DOCUMENTS: Self = Self {
        documents: true,
        sidebar: false,
    };

    pub const SIDEBAR: Self = Self {
        documents: false,
        sidebar: true,
    };
}
