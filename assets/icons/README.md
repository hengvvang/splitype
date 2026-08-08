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
| `explorer/topbar/` | Explorer area top bar (window area header) | `folder`, `link`, `split-h`, `split-v`, `checkmark`, `close`, `maximize`, `restore` |
| `explorer/bottombar/` | Explorer area bottom bar | `folder-plus` |
| `topbar/app_menu/` | Menu buttons in the top bar | `app-menu`, `sun`, `moon`, `checkmark`, `chevron-right` |
| `topbar/chrome/` | Window control buttons (main top bar) | `close`, `minimize`, `maximize`, `restore` |
| `settings/` | Settings window / panel content | `select-chevron`, `checkmark`, `chevron-down`, `chevron-right`, `sun`, `moon`, `plus`, `minus` |
| `settings/topbar/` | Settings area top bar (window area header) | `settings`, `link`, `split-h`, `split-v`, `checkmark`, `close`, `maximize`, `restore` |
| `editor/topbar/` | Editor area top bar (window area header, incl. its dropdown) | `document`, `active`, `link`, `split-h`, `split-v`, `checkmark`, `close`, `maximize`, `restore` |
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
  `icons/topbar/app_menu/checkmark.svg` are separate assets even though the
  files are identical).
- When replacing or restyling an icon, `grep` for the filename to find all
  copies that must be updated together.

## Conventions

- **Naming:** kebab-case, matching the icon's visual (`folder-open.svg`) or
  role (`checkbox-checked.svg`), never `_` or CamelCase.
- **Color:** every SVG keeps `fill="currentColor"` so the app can tint it
  with the active theme via `text_color(...)`.
- **Registration:** a new icon must (1) be added to the directory of the
  surface it belongs to and (2) be mapped in `src/app/assets.rs` under its
  `assets/`-relative key (e.g. `icons/explorer/folder.svg`). Code then
  references it as `svg().path("icons/...")`.

## Provenance and license

Runtime icons are extracted from the **Segoe Fluent Icons** font by
Microsoft — the icon font shipped with Windows 11 and used by WinUI 3 apps.
Glyphs come from the **system font** `C:\Windows\Fonts\SegoeIcons.ttf`
(version 1.53) rather than the older downloadable 1.00 package, so the
shapes match what Windows 11 itself renders. Each local SVG is the font's
glyph converted losslessly from its TrueType outline to a 24×24 SVG path;
codepoints follow the official glyph map:

- Font glyph map (E700–F8CC):
  <https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font>

**License: Segoe Fluent Icons EULA** — the font ships with a Microsoft EULA
that permits using the font and its glyphs to design, develop, and test
programs, but **does not grant redistribution or sublicensing** of the font
or its glyphs to third parties, on any platform. **splitype is a private,
personal project that is not distributed**; the extracted glyphs are used
only on the developer's own Windows machine. If the app is ever published,
the icons must be replaced with a redistributable set.

The extracted SVGs keep `fill="currentColor"` so the app can color icons
with the active splitype theme. Rendering size is controlled by the app via
`size(px(...))`; the `width`/`height` attributes are left as-is and ignored
by the renderer.

Icons with no Segoe counterpart keep their previous **Fluent UI System
Icons** SVG (MIT, [microsoft/fluentui-system-icons](https://github.com/microsoft/fluentui-system-icons)) — see
[Not from the Segoe set](#not-from-the-segoe-set) below.

### Glyph table

Each local file maps to one Segoe glyph. One glyph may feed several surface
directories — an icon is owned by exactly one surface directory per the
[Decoupling](#decoupling) rule, so copies are kept in each surface:

| Local file | Segoe glyph |
| --- | --- |
| `explorer/worktree/folder.svg` | Folder U+E8B7 |
| `explorer/worktree/folder-open.svg` | FolderOpen U+E838 |
| `explorer/bottombar/folder-plus.svg` | NewFolder U+E8F4 |
| `explorer/worktree/markdown.svg`, `editor/outline/markdown.svg` | PinyinIMELogo U+EDE5 |
| `explorer/worktree/file.svg` | Document U+E8A5 |
| `explorer/worktree/chevron-down.svg`, `settings/chevron-down.svg` | ChevronDown U+E70D |
| `explorer/worktree/chevron-right.svg`, `settings/chevron-right.svg`, `topbar/app_menu/chevron-right.svg` | ChevronRight U+E76C |
| `explorer/worktree/view.svg` | View U+E890 |
| `explorer/worktree/hide.svg` | Hide U+ED1A |
| `explorer/worktree/collapse-all.svg` | HideBcc U+E8C5 |
| `explorer/worktree/sync.svg` | Sync U+E895 |
| `explorer/worktree/replace-folder.svg` | SyncFolder U+E8F7 |
| `explorer/topbar/link.svg`, `settings/topbar/link.svg`, `editor/topbar/link.svg` | Link U+E71B |
| `explorer/topbar/split-h.svg`, `settings/topbar/split-h.svg`, `editor/topbar/split-h.svg`, `editor/bottombar/split-h.svg` | ResizeMouseTallMirrored U+EA61 |
| `explorer/topbar/split-v.svg`, `settings/topbar/split-v.svg`, `editor/topbar/split-v.svg`, `editor/bottombar/split-v.svg` | ResizeMouseWide U+E745 |
| `topbar/app_menu/app-menu.svg` | GlobalNavButton U+E700 |
| `settings/plus.svg` | CalculatorAddition U+E948 |
| `settings/minus.svg` | CalculatorSubtract U+E949 |
| `topbar/app_menu/sun.svg`, `settings/sun.svg` | Brightness U+E706 |
| `topbar/app_menu/moon.svg`, `settings/moon.svg` | QuietHours U+E708 |
| `editor/wysiwyg/codeblock/select-chevron.svg`, `settings/select-chevron.svg` | ScrollMode U+ECE7 |
| `editor/wysiwyg/codeblock/line-numbers.svg` | CalculatorPercentage U+E94C |
| `editor/wysiwyg/codeblock/copy.svg` | TaskView U+E7C4 |
| `settings/checkmark.svg`, `editor/bottombar/checkmark.svg`, `topbar/app_menu/checkmark.svg`, `editor/wysiwyg/codeblock/select-checkmark.svg` | CheckMark U+E73E |
| `explorer/topbar/close.svg`, `settings/topbar/close.svg`, `editor/topbar/close.svg`, `editor/bottombar/close.svg`, `topbar/chrome/close.svg` | ChromeClose U+E8BB |
| `explorer/topbar/maximize.svg`, `settings/topbar/maximize.svg`, `editor/topbar/maximize.svg`, `topbar/chrome/maximize.svg` | ChromeMaximize U+E922 |
| `explorer/topbar/restore.svg`, `settings/topbar/restore.svg`, `editor/topbar/restore.svg`, `topbar/chrome/restore.svg` | ChromeRestore U+E923 |
| `topbar/chrome/minimize.svg` | ChromeMinimize U+E921 |
| `editor/topbar/active.svg` | UpdateStatusDot2 U+EC83 |
| `editor/wysiwyg/table/plus.svg` | CalculatorAddition U+E948 |
| `editor/context_menu/plus.svg`, `editor/topbar/add.svg` | CalculatorAddition U+E948 |
| `editor/context_menu/minus.svg` | CalculatorSubtract U+E949 |
| `editor/context_menu/chevron-right.svg` | ChevronRight20 U+F745 |
| `editor/wysiwyg/checkbox.svg`, `editor/preview/checkbox.svg` | Checkbox U+E739 |
| `editor/wysiwyg/checkbox-checked.svg`, `editor/preview/checkbox-checked.svg` | CheckboxCompositeReversed U+E73D |
| `editor/wysiwyg/callout/note.svg` | Info U+E946 |
| `editor/wysiwyg/callout/tip.svg` | Lightbulb U+EA80 |
| `editor/wysiwyg/callout/important.svg` | FavoriteStar U+E734 |
| `editor/wysiwyg/callout/warning.svg` | Warning U+E7BA |

### Not from the Segoe set

These keep their previous **Fluent UI System Icons** SVG (MIT):

- `editor/wysiwyg/callout/caution.svg` — no prohibited glyph in Segoe.
- `editor/wysiwyg/codeblock/select-chevron.svg`, `settings/select-chevron.svg`
