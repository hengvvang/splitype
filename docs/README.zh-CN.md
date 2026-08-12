<p align="right">
  <a href="README.md">English</a> | <a href="docs/README.zh-CN.md">中文</a>
</p>

<h1 align="left">
  Splitype
</h1>

<h3 align="right">
  A fast, native Markdown editor, built with Rust and GPUI.
</h3>


<p align="right">
    <a href="./assets/showcase/showcase.md">Showcase</a>
</p>


## 特性

### 分屏编辑
|       SPLIT                     |
| -------------------------------- |
| ![预览](./assets/showcase/split.png) | 

### 多模态编辑
| 源码编辑                         | 所见即所得编辑                   |              预览                  |
| -------------------------------- | --------------------------------- | -------------------------------- |
| ![预览](./assets/showcase/source_code.png) | ![预览](./assets/showcase/wysiwyg.png) | ![预览](./assets/showcase/preview.png) |



## 快速开始

### 下载预编译版本
在 [Releases](https://github.com/hengvvang/splitype/releases) 页面获取对应平台的构建版本：
- **macOS**：提供 `.app` 应用包或 `.pkg` 安装包（安装包会自动安装并配置 `splitype` 命令行工具）。
- **Windows / Linux**：下载对应平台的压缩包，解压后即可直接运行。

### 从源码构建

确保已安装 Rust 工具链（Edition 2024），然后执行以下命令：

```bash
git clone https://github.com/hengvvang/splitype.git
cd splitype
cargo build --release
```

编译生成的二进制文件位于 `./target/release/splitype`。


## 特别鸣谢

特别感谢 [velotype](https://github.com/manyougz/velotype) 项目及其作者提供的基础代码库，正是它成就了 splitype 的诞生。
特别感谢 [zed](https://github.com/zed-industries/zed) 项目及作者提供的基础代码库，splitype 基本上移植了zed 的 explorer 设计。
特别感谢 [blender](https://github.com/blender/blender) 项目及作者提供的基础代码库，splitype 的分屏设计灵感源自 blender。

## 开源许可证

splitype 基于 [Apache License 2.0](../LICENSE-APACHE) 许可证开源。
