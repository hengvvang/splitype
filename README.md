# splitype

<div align="center">

![splitype banner](./assets/icon/splitype-banner.png)

**A fast, native Markdown editor with WYSIWYG and source-code modes, built with Rust and GPUI.**

[Editor Showcase](./assets/showcase/showcase.md)

[English](README.md) | [中文](docs/README.zh-CN.md)

[![Rust](https://img.shields.io/badge/Rust-2024-f74c00?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/GUI-GPUI%200.2-4b7bec)](https://gpui.rs/)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-2ea44f)](#quick-start)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)

</div>

splitype is a block-based Markdown editor: Markdown is parsed into an editable block tree and rendered natively with [GPUI](https://gpui.rs/) — no Electron, no WebView. Switch between rendered editing (WYSIWYG) and raw source code at any time, and split the editor into as many panes as you need.

## Features

- **🧱 Block model** — Markdown becomes structured, editable blocks; no preview-pane sync loop
- **✍️ Two editing modes** — WYSIWYG rendered editing and raw Markdown source editing
- **🪟 Split layout** — window-level panes (explorer / editor / settings) and editor-inner panels (outline / preview / source / wysiwyg) share one tiled split engine
- **📤 Export** — HTML and PDF export with the active theme mapped into the output
- **🎨 Theme & language packs** — partial-config JSONC files override colors, typography, layout tokens, or UI strings
- **📦 Portable** — single native binary for Windows, Linux, and macOS

## Quick Start

**Download a release** from the [Releases](https://github.com/hengvvang/splitype/releases) page — unzip and run. On macOS, either use the `.app` bundle or the `.pkg` installer (which also sets up the `splitype` CLI command).

**Build from source:**

```bash
git clone https://github.com/hengvvang/splitype.git
cd splitype
cargo build --release
```

## Customization

splitype separates visual themes from UI language packs. A theme file can override global colors, fonts, spacing, menus, dialogs, code highlighting, and layout tokens; missing fields inherit from the built-in base theme (`splitype` or `splitype-light`). Language packs use the same partial-config strategy, falling back to English.

Start with the examples, then import them via **Theme → Add Theme Config** or **Language → Add Language Config**:

- [Custom theme JSONC](assets/custom-theme.example.jsonc)
- [Custom language JSONC](assets/custom-language.example.jsonc)

## Acknowledgements

Special thanks to the [velotype](https://github.com/hengvvang/velotype) project and its author for the original codebase that made this project possible.

## License

splitype is licensed under the [Apache License 2.0](LICENSE-APACHE).
