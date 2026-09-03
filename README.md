<p align="right">
  <a href="README.md">English</a> |
  <a href="docs/README.zh-CN.md">中文</a>
</p>

<h1 align="left">
  Splitype
</h1>

<h3 align="right">
  A fast, native Markdown editor, built with Rust and GPUI.
</h3>

<p align="right">
    <a href="./assets/showcase/showcase.md">Showcase</a> | 
    <a href="https://github.com/hengvvang/splitype/discussions/1">Roadmap</a> | 
    <a href="https://github.com/hengvvang/splitype/wiki">Wiki</a>
</p>

## Features

- **Window Management**: Blender-inspired tiling layout supporting arbitrary splits, resizing, swapping, and edge docking.
- **Multi-Modal Editing**: Seamlessly work across live WYSIWYG (tables, callouts, checklists), multi-cursor source code editing, and synchronized preview.
- **Performance**: Pure Rust & GPUI architecture delivering instant cold startup, low memory usage, and 120+ FPS GPU-accelerated rendering.
- **Customization**: Built-in dark and light themes with configurable typography, editor behaviors, and panel arrangements.

## Installation

### Pre-built Binaries

Pre-compiled binaries and installers for macOS, Windows, and Linux are available on the [Releases](https://github.com/hengvvang/splitype/releases) page. Download the appropriate package for your platform:

- **macOS**: `.dmg`, `.pkg`, or `.app`
- **Windows**: `.msi` installer or portable `.zip`
- **Linux**: `.AppImage`, `.deb`, or `.tar.gz`

### Build from Source

**Prerequisites**: [Rust toolchain](https://rustup.rs/) (stable channel, Edition 2024, MSRV 1.91+).

```bash
# Clone the repository
git clone https://github.com/hengvvang/splitype.git
cd splitype

# Build the release binary
cargo build --release
```

The compiled binary will be located at `./target/release/app` (or `./target/release/app.exe` on Windows).

## Develop

Workspace automation is powered by `cargo xtask`:

- `cargo xtask check` — Run code formatting check, compilation check, and Clippy lints (`--fix` to auto-apply fixes).
- `cargo xtask test` — Run the workspace test suite (accelerated by `cargo-nextest` when available).
- `cargo xtask audit` — Audit dependencies for unused entries (`cargo-machete`) and check security advisories/licenses (`cargo-deny`).
- `cargo xtask ci` — Run the full local CI validation suite in strict mode.
- `cargo xtask dist` — Compile optimized release binaries for distribution.
- `cargo xtask hook` — Install or manage Git pre-commit hooks.

Architecture and design records live in [`docs/develop/architecture.md`](docs/develop/architecture.md) and [`docs/decisions.md`](docs/decisions.md).

## Special Thanks

Splitype is built on the shoulders of these outstanding open-source projects:

- [velotype](https://github.com/manyougz/velotype) — the original codebase that made this project possible.
- [zed](https://github.com/zed-industries/zed) — splitype's file explorer is largely ported from zed's design.
- [blender](https://github.com/blender/blender) — the split-panel layout system is inspired by blender's workspace model.

**Assets & Resources**

- UI Icons by [dreamstale](https://www.flaticon.com/authors/dreamstale) and [smashingstocks](https://www.flaticon.com/authors/smashingstocks), licensed from [Flaticon](https://www.flaticon.com/). UI icons are embedded as SVGs at compile time and tinted dynamically. Redistribution without a valid license is strictly prohibited.
- [Lexend](https://fonts.google.com/specimen/Lexend) typeface by [Thomas Jockin](https://github.com/ThomasJockin), licensed under the [SIL Open Font License 1.1](http://scripts.sil.org/OFL).

## License

Splitype is licensed under the [GNU General Public License v3.0 or later](LICENSE-GPL) ([GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0.html)).

Certain bundled assets are covered by their own licenses:
- Icons: licensed from [Flaticon](https://www.flaticon.com/) — see [Special Thanks](#special-thanks).
- Lexend font: [SIL Open Font License 1.1](http://scripts.sil.org/OFL).
