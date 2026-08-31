pub mod descriptor;
pub mod host;
pub mod id;
pub mod kind;
pub mod registry;
pub mod view;

pub use descriptor::PaneDescriptor;
pub use host::{AutoscrollStrategy, PaneHost, PaneOutlineHost, PaneRenderContext};
pub use id::PaneId;
pub use kind::PaneKind;
pub use registry::PaneRegistry;
pub use view::PaneView;


