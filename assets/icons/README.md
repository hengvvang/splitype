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
| `explorer/worktree/` | Worktree: root row and file tree | `folder`, `open_folder`, `replace_folder`, `file`, `markdown`, `view`, `hide`, `sync_folder`, `collapse-all`, `chevron-down`, `chevron-right`, `file_type_pdf`, `file_type_code`, `file_type_music`, `file_type_image`, `file_type_txt`, `file_type_default` |
| `explorer/topbar/` | Explorer area top bar (window area header) | `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
| `explorer/bottombar/` | Explorer area bottom bar | `new_folder` |
| `titlebar/app_menu/` | Menu buttons in the top bar | `app_menu`, `sun`, `moon`, `checkmark`, `chevron-right` |
| `titlebar/chrome/` | Window control buttons (main top bar) | `close`, `mins`, `maximize`, `restore` |
| `settings/` | Settings window / panel content | `select-chevron`, `checkmark`, `chevron-down`, `chevron-right`, `sun`, `moon`, `plus`, `minus` |
| `settings/topbar/` | Settings area top bar (window area header) | `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
| `editor/topbar/` | Editor area top bar | `add_file`, `active`, `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
| `editor/bottombar/` | Editor panel bottom bar and inner-panel switch menu | `split-h`, `split-v`, `close`, `checkmark` |
| `editor/wysiwyg/` | WYSIWYG panel | `checkbox`, `checkbox-checked` |
| `editor/context_menu/` | Editor right-click menu | `chevron-right`, `plus`, `minus` |
| `editor/wysiwyg/table/` | Table blocks | `plus` |
| `editor/wysiwyg/callout/` | Callout blocks | `note`, `tip`, `important`, `warning`, `caution` |
| `editor/wysiwyg/codeblock/` | Code block toolbar | `copy`, `line-numbers`, `select-checkmark`, `select-chevron` |
| `editor/preview/` | Preview panel | `checkbox`, `checkbox-checked` |
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
  `src/windows/layout/mod.rs`, not as string literals. When renaming or
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
`file_type_icon` extension map in `src/windows/explorer/state.rs`:
markdown, pdf, code, music, image, txt, default.

### Pinned exceptions (not author-supplied)

- `editor/wysiwyg/checkbox.svg`, `editor/preview/checkbox.svg`,
  `editor/wysiwyg/checkbox-checked.svg`,
  `editor/preview/checkbox-checked.svg` — the task-list checkbox look in
  WYSIWYG and Preview is pinned; do not restyle. These come from
  **Fluent UI System Icons** (MIT).
