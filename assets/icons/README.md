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
| `explorer/worktree/` | Worktree: root row and file tree | `folder`, `folder-open`, `replace-folder`, `file`, `markdown`, `view`, `hide`, `sync`, `collapse-all`, `chevron-down`, `chevron-right` |
| `explorer/topbar/` | Explorer area top bar (window area header) | `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
| `explorer/bottombar/` | Explorer area bottom bar | `folder-plus` |
| `titlebar/app_menu/` | Menu buttons in the top bar | `app-menu`, `sun`, `moon`, `checkmark`, `chevron-right` |
| `titlebar/chrome/` | Window control buttons (main top bar) | `close`, `minimize`, `maximize`, `restore` |
| `settings/` | Settings window / panel content | `select-chevron`, `checkmark`, `chevron-down`, `chevron-right`, `sun`, `moon`, `plus`, `minus` |
| `settings/topbar/` | Settings area top bar (window area header) | `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
| `editor/topbar/` | Editor area top bar | `add`, `active`, `check`, `split-h`, `split-v`, `close`, `maximize`, `restore` |
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

- **Naming:** kebab-case, matching the icon's visual (`folder-open.svg`) or
  role (`checkbox-checked.svg`), never `_` or CamelCase.
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

Most runtime icons come from the **Framework7 Icons** set — an iOS-style
monochrome icon font by the Framework7 authors, licensed under
**MIT** (redistributable):

- Upstream: <https://github.com/framework7io/framework7-icons>
- The complete, unmodified `svg/` output of `framework7-icons@5.0.5`
  lives in `../framework7-icons/` (1252 files + LICENSE). **Copy glyphs
  from there into the per-surface directories below, keeping the
  app-facing file names**; never edit the archive in place.
- The archive's own README records the upstream version.

The local files are byte-identical copies of the corresponding upstream
glyph (56×56 viewBox, no `fill`). GPUI ignores the embedded size and tints
via `text_color(...)`, so the shape is what matters.

### Exceptions (not from Framework7)

These keep their previous SVGs because the Framework7 set has no
counterpart, or because their look is intentionally pinned:

- `explorer/worktree/markdown.svg`, `editor/outline/markdown.svg` —
  the Markdown logo has no F7 equivalent.
- `editor/wysiwyg/checkbox.svg`, `editor/preview/checkbox.svg`,
  `editor/wysiwyg/checkbox-checked.svg`,
  `editor/preview/checkbox-checked.svg` — the task-list checkbox look in
  WYSIWYG and Preview is pinned; do not restyle.

The `markdown` files are extracted from the **Segoe Fluent Icons** font
(Microsoft, `C:\Windows\Fonts\SegoeIcons.ttf`); the `checkbox` files come
from **Fluent UI System Icons** (MIT). Both predate the Framework7
migration and are only used for the exceptions above.

### F7 mapping table

| Local file | Framework7 glyph |
| --- | --- |
| `explorer/worktree/folder.svg` | `folder` |
| `explorer/worktree/folder-open.svg` | `folder_fill` |
| `explorer/bottombar/folder-plus.svg` | `folder_fill_badge_plus` |
| `explorer/worktree/file.svg` | `doc_text` |
| `explorer/worktree/chevron-down.svg`, `settings/chevron-down.svg`, `editor/wysiwyg/codeblock/select-chevron.svg`, `settings/select-chevron.svg` | `chevron_down` |
| `explorer/worktree/chevron-right.svg`, `settings/chevron-right.svg`, `titlebar/app_menu/chevron-right.svg`, `editor/context_menu/chevron-right.svg` | `chevron_right` |
| `explorer/worktree/view.svg` | `eye` |
| `explorer/worktree/hide.svg` | `eye_slash` |
| `explorer/worktree/collapse-all.svg` | `arrow_up_to_line` |
| `explorer/worktree/sync.svg` | `arrow_2_circlepath` |
| `explorer/worktree/replace-folder.svg` | `arrow_2_squarepath` |
| `editor/bottombar/split-h.svg` | `square_split_2x1` |
| `editor/bottombar/split-v.svg` | `square_split_1x2` |
| `titlebar/app_menu/app-menu.svg` | `line_horizontal_3` |
| `settings/plus.svg`, `editor/wysiwyg/table/plus.svg`, `editor/context_menu/plus.svg`, `editor/topbar/add.svg` | `plus` |
| `settings/minus.svg`, `editor/context_menu/minus.svg` | `minus` |
| `titlebar/app_menu/sun.svg`, `settings/sun.svg` | `sun_max` |
| `titlebar/app_menu/moon.svg`, `settings/moon.svg` | `moon_fill` |
| `editor/wysiwyg/codeblock/line-numbers.svg` | `number` |
| `editor/wysiwyg/codeblock/copy.svg` | `doc_on_doc` |
| `settings/checkmark.svg`, `editor/bottombar/checkmark.svg`, `titlebar/app_menu/checkmark.svg`, `editor/wysiwyg/codeblock/select-checkmark.svg` | `checkmark` |
| `explorer/topbar/check.svg`, `settings/topbar/check.svg`, `editor/topbar/check.svg` | `checkmark` |
| `explorer/topbar/split-h.svg`, `settings/topbar/split-h.svg`, `editor/topbar/split-h.svg` | `square_split_2x1` |
| `explorer/topbar/split-v.svg`, `settings/topbar/split-v.svg`, `editor/topbar/split-v.svg` | `square_split_1x2` |
| `explorer/topbar/close.svg`, `settings/topbar/close.svg`, `editor/topbar/close.svg`, `editor/bottombar/close.svg`, `titlebar/chrome/close.svg` | `xmark` |
| `explorer/topbar/maximize.svg`, `settings/topbar/maximize.svg`, `editor/topbar/maximize.svg`, `titlebar/chrome/maximize.svg` | `arrow_up_left_arrow_down_right` |
| `explorer/topbar/restore.svg`, `settings/topbar/restore.svg`, `editor/topbar/restore.svg`, `titlebar/chrome/restore.svg` | `arrow_down_right_arrow_up_left` |
| `editor/topbar/active.svg` | `link` (active-editor marker) |
| `titlebar/chrome/minimize.svg` | `minus` |
| `editor/wysiwyg/callout/note.svg` | `quote_bubble` |
| `editor/wysiwyg/callout/tip.svg` | `lightbulb` |
| `editor/wysiwyg/callout/important.svg` | `exclamationmark_circle` |
| `editor/wysiwyg/callout/warning.svg` | `exclamationmark_triangle` |
| `editor/wysiwyg/callout/caution.svg` | `xmark_octagon` |
