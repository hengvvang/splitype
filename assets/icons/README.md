# Icons

Runtime UI icons for splitype: small monochrome SVGs that are embedded into
the binary at build time (via `include_bytes!` in `src/app/assets.rs`) and
rendered through the GPUI asset source. They are not loaded from disk at
runtime and never ship as loose files.

## Structure

Shell-level icons live in this global `assets/icons/` directory. Plugin-specific icons are co-located within each plugin crate (`crates/explorer/assets/icons/`, `crates/settings/assets/icons/`, `crates/editor/assets/icons/`) and registered via their respective crate `assets` modules.

| Directory | Surface | Icons |
| --- | --- | --- |
| `crates/explorer/assets/icons/` | Explorer: worktree, panel, topbar, bottombar | `worktree/*`, `topbar/*`, `bottombar/*`, `panel.svg` |
| `crates/settings/assets/icons/` | Settings: form controls, panel, topbar | Form controls (`select-chevron`, `checkmark`, etc.), `topbar/*`, `panel.svg` |
| `crates/editor/assets/icons/` | Editor: topbar, bottombar, context_menu, wysiwyg, preview, outline | `topbar/*`, `bottombar/*`, `context_menu/*`, `wysiwyg/*`, `preview/*`, `outline/*`, `panel.svg` |
| `assets/icons/titlebar/app_menu/` | Menu buttons in the main window top bar | `app_menu`, `sun`, `moon`, `checkmark`, `chevron-right` |
| `assets/icons/titlebar/chrome/` | Window control buttons (main title bar) | `close`, `mins`, `maximize`, `restore` |
| `assets/icons/chrome/` | Window chrome (kind-independent) | `check`, `missing` |
| `assets/icons/splitter/` | Window & pane split gesture overlays | `arrow-up`, `arrow-down`, `arrow-left`, `arrow-right`, `dock-up`, `dock-down`, `dock-left`, `dock-right`, `split-area`, `swap` |
| `assets/icons/emoji/` | About dialog emoji icons | `1.svg` - `18.svg` |

## Decoupling

**An icon is owned by exactly one surface directory; if the same icon is
rendered in N places, it exists as N copies, one in each surface's
directory.**

- Never reference an icon from another surface's directory — every call
  site uses its own surface's copy.
- This is a deliberate trade-off: duplicate files over cross-surface
  coupling, so that changing one surface's icon can never affect another
  surface, and each surface directory stays a complete, self-contained
  inventory of what it renders.
- When an icon is copied, register every copy in `src/app/assets.rs` under
  its own key (e.g. `icons/settings/checkmark.svg` and
  `icons/titlebar/app_menu/checkmark.svg` are separate assets even though the
  files are identical).
- When replacing or restyling an icon, `grep` for the filename to find all
  copies that must be updated together.

## Conventions

- **Naming:** the icon's role (`checkbox-checked.svg`) or the author's
  chosen name; hand-supplied icons keep their original file name
  (`open_folder.svg`, `replace_folder.svg`, `file_type_pdf.svg`).
- **Color:** every SVG keeps `fill="currentColor"` (or no `fill` at all —
  GPUI renders SVGs as an alpha mask and tints them with `text_color(...)`),
  so the app can color icons with the active theme.
- **Registration:** a new icon must (1) be added to the directory of the
  surface it belongs to and (2) be mapped in `src/app/assets.rs` under its
  `assets/`-relative key (e.g. `icons/explorer/folder.svg`). Code then
  references it as `svg().path("icons/...")`.
- **Dynamic paths:** area top bar icons (`icons/{area}/topbar/{name}.svg`)
  are referenced through `area_topbar_icon(kind, name)` in
  `src/editor/window_layout.rs`, not as string literals. When renaming or
  deleting files in the three `topbar/` directories, search for the `name`
  argument (e.g. `"check"`, `"split-h"`) instead of the full path.

## Provenance and license

### Author-supplied icons

All runtime icons are **author-supplied SVGs** (Illustrator-exported or
hand-drawn, flat monochrome style). They are the single source of truth;
there is no upstream icon library in the repository. GPUI renders them as
alpha masks and tints them with the active theme via `text_color(...)`, so
embedded sizes, viewBoxes and fill colors do not matter.

File-type icons (`explorer/worktree/file_type_*.svg`) are selected by the
`file_type_icon` extension map in `src/editor/explorer/state.rs`:
markdown, pdf, code, music, image, txt, default.

### Pinned exceptions (not author-supplied)

- `editor/wysiwyg/checkbox.svg`, `editor/preview/checkbox.svg`,
  `editor/wysiwyg/checkbox-checked.svg`,
  `editor/preview/checkbox-checked.svg` — the task-list checkbox look in
  WYSIWYG and Preview is pinned; do not restyle. These come from
  **Fluent UI System Icons** (MIT).
