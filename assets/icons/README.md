# Icons

Runtime UI icons for splitype: small monochrome SVGs that are embedded into
the binary at build time (via `include_bytes!` in `src/app/assets.rs`) and
rendered through the GPUI asset source. They are not loaded from disk at
runtime and never ship as loose files.

## Structure

Icons are grouped by the UI surface they render in, mirroring the app's
window and panel design. A surface owns every icon it renders — an icon
used by several surfaces exists once per surface (see
[Decoupling](#decoupling) below).

| Directory | Surface | Icons |
| --- | --- | --- |
| `explorer/worktree/` | Worktree: root row and file tree | `folder`, `folder-open`, `file`, `file-plus`, `markdown`, `eye`, `eye-off`, `refresh`, `collapse-all`, `chevron-down`, `chevron-right` |
| `explorer/topbar/` | Explorer area top bar (window area header) | `link`, `split-h`, `split-v`, `check`, `close`, `maximize`, `restore` |
| `explorer/bottombar/` | Explorer area bottom bar | `folder-plus` |
| `topbar/app_menu/` | Menu buttons in the top bar | `sun`, `moon`, `check`, `chevron-right` |
| `topbar/chrome/` | Window control buttons (main top bar) | `close`, `minimize`, `maximize`, `restore` |
| `settings/` | Settings window / panel content | `select-chevron`, `check`, `chevron-down`, `chevron-right`, `sun`, `moon` |
| `settings/topbar/` | Settings area top bar (window area header) | `link`, `split-h`, `split-v`, `check`, `close`, `maximize`, `restore` |
| `editor/topbar/` | Editor area top bar (window area header, incl. its dropdown) | `link`, `split-h`, `split-v`, `check`, `close`, `maximize`, `restore` |
| `editor/bottombar/` | Editor panel bottom bar and inner-panel switch menu | `split-h`, `split-v`, `close`, `check` |
| `editor/wysiwyg/` | WYSIWYG panel | `task-check` |
| `editor/wysiwyg/table/` | Table blocks | `plus`, `handle-row`, `handle-row-hollow`, `handle-row-solid`, `handle-column` |
| `editor/wysiwyg/callout/` | Callout blocks | `note`, `tip`, `important`, `warning`, `caution` |
| `editor/wysiwyg/codeblock/` | Code block toolbar | `line-numbers`, `select-check`, `select-chevron` |
| `editor/preview/` | Preview panel | `task-check` |
| `editor/outline/` | Outline panel | `markdown` |

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
  its own key (e.g. `icons/settings/check.svg` and
  `icons/topbar/app_menu/check.svg` are separate assets even though the
  files are identical).
- When replacing or restyling an icon, `grep` for the filename to find all
  copies that must be updated together.

## Conventions

- **Naming:** kebab-case, matching the icon's visual (`folder-open.svg`) or
  role (`task-check.svg`), never `_` or CamelCase.
- **Color:** every SVG keeps `fill="currentColor"` so the app can tint it
  with the active theme via `text_color(...)`.
- **Registration:** a new icon must (1) be added to the directory of the
  surface it belongs to and (2) be mapped in `src/app/assets.rs` under its
  `assets/`-relative key (e.g. `icons/explorer/folder.svg`). Code then
  references it as `svg().path("icons/...")`.

## Provenance and license

These SVG icons are sourced from Iconify and stored locally so the app can
embed them through the GPUI asset source at build time.

| Local file | Iconify icon | Icon set | License |
| --- | --- | --- | --- |
| `explorer/folder.svg` | [`material-symbols:folder`](https://icon-sets.iconify.design/material-symbols/folder/) | Material Symbols | Apache-2.0 |
| `explorer/markdown.svg` (also `editor/outline/markdown.svg`) | [`mdi:language-markdown`](https://icon-sets.iconify.design/mdi/language-markdown/) | Material Design Icons | Apache-2.0 |
| `topbar/chrome/close.svg` (also `editor/close.svg`) | [`codicon:chrome-close`](https://icon-sets.iconify.design/codicon/chrome-close/) | Codicons by Microsoft Corporation | CC BY 4.0 |
| `topbar/chrome/minimize.svg` | [`codicon:chrome-minimize`](https://icon-sets.iconify.design/codicon/chrome-minimize/) | Codicons by Microsoft Corporation | CC BY 4.0 |
| `topbar/chrome/maximize.svg` (also `editor/maximize.svg`) | [`codicon:chrome-maximize`](https://icon-sets.iconify.design/codicon/chrome-maximize/) | Codicons by Microsoft Corporation | CC BY 4.0 |
| `topbar/chrome/restore.svg` (also `editor/restore.svg`) | [`codicon:chrome-restore`](https://icon-sets.iconify.design/codicon/chrome-restore/) | Codicons by Microsoft Corporation | CC BY 4.0 |

The exported SVGs keep `fill="currentColor"` so the app can color icons with
the active splitype theme.
