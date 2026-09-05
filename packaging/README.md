# Packaging

Platform-specific packaging metadata, OS-level manifests, installer configurations, and native desktop integration files for **splitype**.

This directory holds the build-time and distribution-time assets needed to package splitype for target operating systems. Runtime assets (such as UI icons and theme templates) live in [`assets/`](../assets/).

---

## Directory Layout

```text
packaging/
├── windows/
│   ├── build_dist.ps1              # Standalone packaging script to assemble dist/splitype-windows-x64
│   ├── splitype.ico                # Multi-resolution icon embedded into binary and staged for dist
│   ├── splitype.manifest           # Windows application manifest (UTF-8, Long paths, PerMonitorV2 DPI)
│   └── splitype.rc                 # PE resource script embedding icon into executable
├── macos/
│   ├── build_app.sh                # Standalone script to assemble dist/splitype.app
│   ├── build_pkg.sh                # Script to create installable splitype.pkg package
│   ├── Info.plist                  # macOS .app bundle property list & document associations
│   ├── splitype.icns               # Apple Icon format bundle icon
│   └── pkg/                        # macOS PKG graphical installer scripts
│       ├── Distribution.xml        # productbuild multi-package distribution spec
│       ├── postinstall             # Post-installation hook (creates /usr/local/bin symlink)
│       └── preuninstall            # Pre-removal hook (cleans up /usr/local/bin symlink)
└── linux/
    ├── com.hengvvang.splitype.desktop # FreeDesktop XDG application entry & MIME bindings
    └── icons/hicolor/              # Standard FreeDesktop hicolor theme icons
        ├── 256x256/apps/com.hengvvang.splitype.png
        └── 512x512/apps/com.hengvvang.splitype.png
```

---

## Platform Details

### 1. Windows (`windows/`)
- **`build_dist.ps1`**: Compiles release binary and packages `dist/splitype-windows-x64/` (and optional `.zip` distribution archive).
- **`splitype.ico`**: 16, 32, 48, 64, 128, and 256 px resolutions.
- **`splitype.manifest`**: Application manifest deployed side-by-side (`splitype.exe.manifest`) declaring:
  - `Per-Monitor V2` DPI awareness (prevents scaling blur on high-DPI displays).
  - Native UTF-8 active code page (`activeCodePage`).
  - Long path awareness (`longPathAware`).
  - Windows 10 & 11 compatibility.
  - Common Controls v6 visual styling.
- **`splitype.rc`**: Resource script embedding `splitype.ico` into the PE binary table.

### 2. macOS (`macos/`)
- **`build_app.sh`**: Compiles release binary and packages `dist/splitype.app`.
- **`build_pkg.sh`**: Uses `pkgbuild` and `productbuild` to generate `dist/splitype-<version>.pkg`.
- **`splitype.icns`**: Native macOS multi-resolution icon bundle.
- **`Info.plist`**: Bundle metadata defining `CFBundleIdentifier` (`com.hengvvang.splitype`), executable name, minimum macOS version (`10.15`), and associated document types (`net.daringfireball.markdown`, `public.plain-text`).
- **`pkg/`**: Packaging definitions for creating signed, installable `.pkg` distributions with automatic `/usr/local/bin/splitype` CLI symlinking.

### 3. Linux (`linux/`)
- **`com.hengvvang.splitype.desktop`**: Desktop entry defining categories (`Office;TextEditor;Utility;`), MIME types (`text/markdown;text/x-markdown;`), and `StartupWMClass` for correct taskbar grouping.
- **`icons/hicolor/`**: System launcher and notification icons placed in standard system icon paths (`/usr/share/icons/hicolor/...`).

---

## Regenerating Icons

All platform icons (`.ico`, `.icns`, and Linux hicolor PNGs) are generated automatically from `scripts/tools/icon-gen/logo.svg`. Whenever `logo.svg` is modified:

```bash
cd scripts/tools/icon-gen
cargo run --release
```
