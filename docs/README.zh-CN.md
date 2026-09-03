<p align="right">
  <a href="../README.md">English</a> |
  <a href="README.zh-CN.md">中文</a>
</p>

<h1 align="left">
  Splitype
</h1>

<h3 align="right">
  基于 Rust 与 GPUI 构建的高性能原生 Markdown 编辑器。
</h3>

<p align="right">
    <a href="../assets/showcase/showcase.md">Showcase</a> | 
    <a href="https://github.com/hengvvang/splitype/discussions/1">Roadmap</a> | 
    <a href="https://github.com/hengvvang/splitype/wiki">Wiki</a>
</p>

## 特性

- **窗口管理**：受 Blender 启发的平铺布局系统，支持任意横纵分屏、拖拽缩放、面板交换与边缘停靠。
- **多模式编辑**：无缝支持所见即所得（含表格、提示块、任务清单）、多光标源码编辑与同步高保真预览。
- **性能**：基于纯 Rust 与 GPUI 架构，带来瞬时冷启动、极低内存占用与 120+ FPS GPU 加速流畅渲染。
- **自定义**：内置浅色与深色主题，支持字体排版、编辑器行为与面板布局的自由定制。

## 安装

### 预编译安装包

适用于 macOS、Windows 和 Linux 的预编译二进制文件与安装包可在 [Releases](https://github.com/hengvvang/splitype/releases) 页面获取。请根据你的平台下载对应的软件包：

- **macOS**：`.dmg`、`.pkg` 或 `.app`
- **Windows**：`.msi` 安装包或便携版 `.zip`
- **Linux**：`.AppImage`、`.deb` 或 `.tar.gz`

### 从源码构建

**环境要求**：[Rust 工具链](https://rustup.rs/)（stable 版本，Edition 2024，MSRV 1.91+）。

```bash
# 克隆代码仓库
git clone https://github.com/hengvvang/splitype.git
cd splitype

# 编译发布版本二进制
cargo build --release
```

编译生成的二进制文件位于 `./target/release/app`（Windows 平台为 `./target/release/app.exe`）。

## 开发

工作区工程自动化由 `cargo xtask` 驱动：

- `cargo xtask check` — 运行代码格式化检查、编译检查及 Clippy 代码分析（支持 `--fix` 自动修复）。
- `cargo xtask test` — 运行工作区测试套件（检测到 `cargo-nextest` 时自动并行加速）。
- `cargo xtask audit` — 审计未使用的依赖（`cargo-machete`）及安全漏洞与开源许可合规性（`cargo-deny`）。
- `cargo xtask ci` — 在本地严格模式下运行完整的 CI 验证流程。
- `cargo xtask dist` — 编译用于分发的优化发行包。
- `cargo xtask hook` — 安装与管理 Git pre-commit 钩子。

架构设计与技术决策记录详见 [`docs/develop/architecture.md`](develop/architecture.md) 与 [`docs/decisions.md`](decisions.md)。

## 特别鸣谢

Splitype 的诞生离不开以下优秀的开源项目：

- [velotype](https://github.com/manyougz/velotype) — 本项目的原始代码库，奠定了 splitype 的基础。
- [zed](https://github.com/zed-industries/zed) — splitype 的文件资源管理器大量移植自 zed 的设计。
- [blender](https://github.com/blender/blender) — splitype 的分屏布局系统受 blender 工作区模型启发。

**资产与资源**

- UI 图标由 [dreamstale](https://www.flaticon.com/authors/dreamstale) 和 [smashingstocks](https://www.flaticon.com/authors/smashingstocks) 设计，经 [Flaticon](https://www.flaticon.com/) 授权使用。UI 图标在编译期作为 SVG 资源嵌入二进制并支持主题色彩动态渲染。未经许可，严禁私自再分发。
- [Lexend](https://fonts.google.com/specimen/Lexend) 字体由 [Thomas Jockin](https://github.com/ThomasJockin) 设计，基于 [SIL Open Font License 1.1](http://scripts.sil.org/OFL) 许可发布。

## 开源许可

Splitype 基于 [GNU 通用公共许可证 3.0 或更新版本](../LICENSE-GPL)（[GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0.html)）发布。

部分内置资产适用其自身的许可协议：
- 图标：经 [Flaticon](https://www.flaticon.com/) 授权使用 — 详见 [特别鸣谢](#特别鸣谢)。
- Lexend 字体：[SIL Open Font License 1.1](http://scripts.sil.org/OFL)。
