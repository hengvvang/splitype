#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabKind {
    #[default]
    Transient,
    Persistent,
}
