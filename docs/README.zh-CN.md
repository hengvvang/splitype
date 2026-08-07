# splitype

<div align="center">

![splitype banner](../assets/identity/banner.png)

**一款基于 Rust 与 GPUI 构建的快速原生 Markdown 编辑器，支持所见即所得与源码双模式。**

[编辑器展示](../assets/showcase/showcase.md)

[English](../README.md) | [中文](README.zh-CN.md)

[![Rust](https://img.shields.io/badge/Rust-2024-f74c00?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/GUI-GPUI%200.2-4b7bec)](https://gpui.rs/)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-2ea44f)](#快速开始)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../LICENSE-APACHE)

</div>

splitype 是一款块级 Markdown 编辑器：Markdown 被解析为可编辑的块树，并由 [GPUI](https://gpui.rs/) 原生渲染——不依赖 Electron 或任何 WebView。你可以在渲染编辑（WYSIWYG）与原始源码模式之间随时切换，并按需将编辑器拆分为任意多个分屏。

## 特性

- **🧱 Block 模型** — Markdown 变为结构化的可编辑块，无需预览窗同步循环
- **✍️ 双编辑模式** — 所见即所得渲染编辑与 Markdown 源码编辑
- **🪟 分屏布局** — 窗口级面板（资源管理器 / 编辑器 / 设置）与编辑器内面板（大纲 / 预览 / 源码 / 所见即所得）共用同一套平铺分屏引擎
- **📤 导出** — 支持 HTML 与 PDF 导出，当前主题会映射到输出中
- **🎨 主题与语言包** — 通过局部配置的 JSONC 文件覆盖颜色、排版、布局 token 或界面文案
- **📦 便携** — 面向 Windows、Linux、macOS 的单一原生可执行文件

## 快速开始

**下载 release**：从 [Releases](https://github.com/hengvvang/splitype/releases) 页面获取对应平台的构建，解压即可运行。macOS 用户可以使用 `.app` 应用包，或使用 `.pkg` 安装包（会自动配置 `splitype` 命令行工具）。

**从源码构建：**

```bash
git clone https://github.com/hengvvang/splitype.git
cd splitype
cargo build --release
```

## 自定义

splitype 将视觉主题与界面语言包分开管理。主题文件可以覆盖全局颜色、字体、间距、菜单、弹窗、代码高亮与布局 token；缺失字段继承自内置基准主题（`splitype` 或 `splitype-light`）。语言包采用相同的局部配置策略，缺失文案回退到英文。

从示例文件开始，然后在应用内通过 **主题 → 添加主题配置** 或 **语言 → 添加语言配置** 导入：

- [自定义主题 JSONC](../assets/examples/custom-theme.example.jsonc)
- [自定义语言 JSONC](../assets/examples/custom-language.example.jsonc)

## 特别鸣谢

特别感谢 [velotype](https://github.com/hengvvang/velotype) 项目及其作者提供的原始代码库，正是它成就了本项目的诞生。

## 许可证

splitype 使用 [Apache License 2.0](../LICENSE-APACHE)。
