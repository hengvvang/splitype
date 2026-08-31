#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabKind {
    #[default]
    Transient,
    Persistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OpenFileMode {
    #[default]
    Transient,
    Persistent,
}
