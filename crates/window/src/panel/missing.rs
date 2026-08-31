//! Placeholder panel for kinds whose plugin is unavailable.
//!
//! When a layout leaf references a kind with no registered descriptor (for
//! example a restored window whose plugin was removed), the shell shows this
//! placeholder instead of a blank tile. The placeholder keeps the layout
//! intact and names the owning plugin when its manifest is known.

use core_contracts::{PanelCapabilities, PanelId, PanelKind, PanelRenderContext, PanelView};
use gpui::*;
use std::any::Any;

/// Placeholder shown for a panel kind whose plugin is not available.
pub struct MissingPanelView {
    panel_id: PanelId,
    kind: PanelKind,
    /// Display name resolved from the owning plugin's manifest, or the raw
    /// kind when no manifest knows this kind.
    display_name: SharedString,
}

impl MissingPanelView {
    /// Builds a placeholder for `kind`, resolving the display name through
    /// the plugin registry.
    pub fn new(panel_id: PanelId, kind: PanelKind) -> Self {
        let display_name = core_contracts::PluginRegistry::panel_kind_owner_global(kind.clone())
            .ok()
            .flatten()
            .and_then(|plugin_id| {
                core_contracts::PluginRegistry::registered(plugin_id)
                    .ok()
                    .flatten()
            })
            .map(|manifest| SharedString::from(manifest.name.as_str()))
            .unwrap_or_else(|| SharedString::from(kind.as_str()));
        Self {
            panel_id,
            kind,
            display_name,
        }
    }
}

impl PanelView for MissingPanelView {
    fn kind(&self) -> PanelKind {
        self.kind.clone()
    }

    fn display_name(&self) -> SharedString {
        self.display_name.clone()
    }

    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::NONE
    }

    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> AnyElement {
        let c = &ctx.theme.colors;
        let d = &ctx.theme.dimensions;
        let t = &ctx.theme.typography;

        div()
            .id(("missing-panel", self.panel_id.0))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .bg(c.dialog_surface)
            .rounded(px(d.panel_tile_radius))
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .child(
                svg()
                    .path("icons/chrome/missing.svg")
                    .size(px(28.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(t.dialog_title_size))
                    .font_weight(t.dialog_title_weight.to_font_weight())
                    .text_color(c.dialog_title)
                    .child(self.display_name.clone()),
            )
            .child(
                div()
                    .text_size(px(t.dialog_body_size))
                    .text_color(c.dialog_muted)
                    .child(format!("Plugin for '{}' is not available.", self.kind)),
            )
            .into_any_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::panel::MissingPanelView;
    use core_contracts::{PanelId, PanelKind, PanelView};
    use gpui::SharedString;

    #[test]
    fn missing_panel_resolves_owner_display_name() {
        let manifest = core_contracts::PluginManifest {
            manifest_version: core_contracts::PLUGIN_MANIFEST_VERSION,
            plugin: core_contracts::PluginId::from_static("com.example.missing-test"),
            name: "Missing Test".into(),
            version: "0.1.0".into(),
            description: None,
            entry: core_contracts::PluginEntry::InProcess {
                registration: "test".into(),
            },
            capabilities: core_contracts::PluginCapabilities {
                panes: Vec::new(),
                panels: vec![PanelKind::from_static("com.example.missing-test.panel")],
            },
            resources: core_contracts::PluginResources::default(),
            commands: Vec::new(),
        };
        core_contracts::PluginRegistry::register_global(manifest).expect("register manifest");

        let view = MissingPanelView::new(
            PanelId(1),
            PanelKind::from_static("com.example.missing-test.panel"),
        );
        assert_eq!(view.display_name(), SharedString::from("Missing Test"));
        assert_eq!(
            view.kind(),
            PanelKind::from_static("com.example.missing-test.panel")
        );
    }
}
