<p align="right">
  <a href="README.md">English</a> | <a href="docs/README.zh-CN.md">中文</a>
</p>

<h1 align="left">
  Splitype
</h1>

<p align="center">
  <img src="../assets/identity/banner.png" alt="Splitype" />
</p>

<h3 align="right">
  A fast, native Markdown editor, built with Rust and GPUI.
</h3>

<p align="right">
    <a href="./assets/showcase/showcase.md">Showcase</a> | 
    <a href="https://github.com/hengvvang/splitype/discussions/1">Roadmap</a> | 
    <a href="https://github.com/hengvvang/splitype/wiki">Wiki</a>
</p>



## 特性

### 分屏编辑
> Splitype的分屏功能允许你根据你的喜好自由打造自己的专属工作区

|       SPLIT                     |
| -------------------------------- |
| ![预览](../assets/showcase/split.png) | 


### 多模态编辑
> Splitype 分屏预览编辑和实时编辑两种工作流

| 分屏编辑                         | 所见即所得编辑                   |
| -------------------------------- | -------------------------------- |
| ![预览](../assets/showcase/workflow_source-preview.png) | ![预览](../assets/showcase/workflow_wysiwyg.png) |



## 快速开始

### 下载预编译版本
在 [Releases](https://github.com/hengvvang/splitype/releases) 页面获取对应平台的构建版本：
- **macOS**：提供 `.app` 应用包或 `.pkg` 安装包（安装包会自动安装并配置 `splitype` 命令行工具）。
- **Windows**：下载对应平台的压缩包，解压后即可直接运行。
- **Linux**：下载对应平台的压缩包，解压后即可直接运行。

### 从源码构建

确保已安装 Rust 工具链（Edition 2024），然后执行以下命令：

```bash
git clone https://github.com/hengvvang/splitype.git
cd splitype
cargo build --release
```

编译生成的二进制文件位于 `./target/release/splitype`。



## 特别鸣谢

- 特别感谢 [velotype](https://github.com/manyougz/velotype) 项目及其作者提供的基础代码库，正是它成就了 splitype 的诞生。
- 特别感谢 [zed](https://github.com/zed-industries/zed) 项目及作者提供的基础代码库，splitype 基本上移植了zed 的 explorer 设计。
- 特别感谢 [blender](https://github.com/blender/blender) 项目及作者提供的基础代码库，splitype 的分屏设计灵感源自 blender。
- 特别感谢 [dreamstale](https://www.flaticon.com/authors/dreamstale) 设计的icons，splitype 在 [flaticon](https://www.flaticon.com/) 上获得授权并使用，任何个人或者组织严禁在没有授权许可的情况下私自下载使用！



## 开源许可证

Splitype 支持多种开源许可协议（由您自由选择）：

- [GNU 通用公共许可证 3.0 或更新版本](../LICENSE-GPL) ([GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0.html))
- [MIT 许可证](../LICENSE-MIT) ([MIT](https://opensource.org/licenses/MIT))
- [Apache 许可证 2.0 版本](../LICENSE-APACHE) ([Apache-2.0](http://www.apache.org/licenses/LICENSE-2.0))
