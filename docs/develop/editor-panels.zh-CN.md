# 编辑器内部面板 —— 布局、标签页与聚焦

编辑器内部面板系统的设计说明：面板的分裂树布局、分屏的创建方式、面板如何与激活标签页绑定，以及聚焦面板如何驱动状态栏。

## 概述

编辑器窗口承载一个或多个**内部面板**（视图）。所有面板渲染**同一个激活标签页**——面板布局负责*结构*，激活标签页负责*数据*，聚焦面板负责*状态栏操作的目标*。三者完全解耦：

| 关注点 | 归属 |
| --- | --- |
| 布局结构 | `WindowLayout.editor_inner_panel_layouts`（分裂树）|
| 数据源 | 编辑器的激活标签页 |
| 操作目标 | `WindowLayout.focused_editor_inner_panel` |

## 数据模型

```rust
// 每个窗口级区域一棵分裂树（编辑器区域内部包含面板）。
editor_inner_panel_layouts: HashMap<area_id, SplitTree<EditorInnerPanelKind>>

// 面板类型。
enum EditorInnerPanelKind { Wysiwyg, SourceCode, Preview, Outline }

// 递归二叉分裂树。
enum SplitTree<T> {
    Leaf { id: usize, kind: T },
    Split { id: usize, direction: Axis, ratio: f32, first: Box<SplitTree<T>>, second: Box<SplitTree<T>> },
}

// 全局唯一的聚焦面板（不是每个区域一个）。
focused_editor_inner_panel: Option<(area_id, panel_id)>
```

以上全部属于编辑器级状态（`Editor.panels.layout`），因此布局在切换标签页时保持不变——切换标签页只改变所有面板渲染的文档。

## 分屏操作

### 两个分屏入口

| 入口 | 函数 | 新面板类型 |
| --- | --- | --- |
| 状态栏分屏按钮（横向 / 纵向）| `split_editor_inner_panel` | 继承**聚焦**面板的类型，比例为 `0.5` |
| 面板四角拖动 | `split_editor_inner_panel_with_ratio` | 继承**被拖动**面板的类型，比例来自拖拽手势 |

如果无法解析目标面板的类型（正常不应发生），新面板回退为 `SourceCode`。

### 调整操作

| 操作 | 函数 |
| --- | --- |
| 拖动分割条 | `update_editor_inner_panel_splitter_drag` —— 仅改变比例 |
| 关闭面板 | `close_editor_inner_panel` —— 仅当剩余叶子数大于 1 时才移除 |
| 切换面板类型 | `change_editor_inner_panel_kind` —— 通过面板头部下拉菜单 |

## 标签页绑定

- 所有面板渲染**激活标签页**的文档：
  - `Wysiwyg` 使用激活标签页的主要渲染内容。
  - `SourceCode` 编辑同一文档的视图（编辑会同步回文档）。
  - `Preview` / `Outline` 由同一文档派生。
- 分裂树是编辑器级状态：切换标签页时不会重置。
- 欢迎状态（无标签页）：所有面板显示欢迎提示；分屏仍然可用，可以在打开文档前预先布置好布局。
- 依赖文档的状态栏项（类型按钮、光标位置、字数）在无激活标签页时隐藏。

## 聚焦设计

```rust
focused_editor_inner_panel: Option<(area_id, panel_id)>
```

- **全局唯一**：整个编辑器恰好有一个聚焦面板。
- **设置时机**：点击面板时（`on_mouse_down`）；首次渲染时自动聚焦第一个面板；关闭聚焦面板后，下一次渲染自动聚焦剩余的第一个。
- **消费方**是匹配区域的状态栏（它通过 `area_id` 过滤全局聚焦）：
  - **类型按钮** —— 显示聚焦面板的类型；点击打开该面板的类型切换下拉菜单。
  - **分屏按钮** —— 以聚焦面板为目标；新面板继承其类型。
  - **关闭按钮** —— 关闭聚焦面板（仅在多面板时显示）。

## 端到端流程

```
点击面板           → focused = (area_id, panel_id)
状态栏（该区域）   → 显示聚焦面板信息
分屏按钮           → 新面板继承聚焦类型，以 0.5 比例插入
四角拖动           → 新面板继承被拖动类型，以手势比例插入
所有面板渲染       → 同一个激活标签页文档
切换标签页         → 布局不变；所有面板切换到新文档
```

## 代码位置

| 关注点 | 文件 |
| --- | --- |
| 布局状态（分裂树、聚焦、拖拽会话）| `src/layout/state.rs` |
| 分裂树操作 | `src/layout/tree.rs` |
| 面板类型 | `src/layout/types.rs` |
| 内部面板渲染 | `src/editor/panels/layout/mod.rs` |
| 状态栏按钮与聚焦显示 | `src/windows/editor/status_bar.rs` |
