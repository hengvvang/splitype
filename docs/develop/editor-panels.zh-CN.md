# 编辑器区域、内部面板与模式迁移

编辑器内部布局系统的设计说明：窗口级区域、每个区域的编辑器会话（标签页 + 面板分裂树）、面板类型所编码的欢迎/编辑模式模型，以及聚焦面板如何驱动状态栏。

## 概述

窗口被划分为若干**区域**（`window_area_tree`），每个区域有一个类型（`Explorer` / `Settings` / `Editor`）。一个 `Editor` 区域拥有一个**编辑器会话**——它自己的标签页列表 *和* 它自己的内部面板分裂树。这是刻意设计：每个编辑器区域完全独立（标签页独立、面板布局独立），explorer 打开文件只作用于**激活编辑器**。

在编辑器区域内，所有面板渲染该区域的**同一个激活标签页**——面板布局负责*结构*，激活标签页负责*数据*，聚焦面板负责*状态栏操作的目标*。

| 关注点 | 归属 |
| --- | --- |
| 窗口区域布局 | `WindowLayout.window_area_tree` |
| 每区域的标签页 + 面板树 | `WindowLayout.editor_sessions` |
| 面板排列 | `EditorSession.inner_panel_tree`（分裂树）|
| 数据源 | 区域的激活标签页（`EditorSession.tab_list`）|
| Explorer 打开文件的目标 | `WindowLayout.active_editor_area`（仅前台）|
| 状态栏操作目标 | `WindowLayout.focused_editor_inner_panel` |

## 数据模型

```rust
// 每个窗口区域一个编辑器会话。会话聚合一个编辑器区域拥有的两样东西，
// 使它们永远不会漂移。
struct EditorSession {
    tab_list: EditorTabList,                              // 该区域的标签页
    inner_panel_tree: SplitTree<EditorInnerPanelKind>,    // 该区域的面板
}
editor_sessions: HashMap<AreaId, EditorSession>

// 面板类型：外层是模式，内层是该模式下的面板类型。树即真相。
enum EditorInnerPanelKind {
    Welcome(WelcomePanelKind),   // 欢迎模式（无标签页）
    Editing(EditingPanelKind),   // 编辑模式（有标签页）
}
enum WelcomePanelKind { Welcome(Option<EditingPanelKind>) }
enum EditingPanelKind { SourceCode, Wysiwyg, Preview, Outline }

// 递归二叉分裂树。
enum SplitTree<T> {
    Leaf { id: usize, kind: T },
    Split { id: usize, direction: Axis, ratio: f32, first: Box<SplitTree<T>>, second: Box<SplitTree<T>> },
}

// 全局唯一的聚焦面板（不是每个区域一个）。
focused_editor_inner_panel: Option<InnerPanelLocation>   // { area_id, panel_id }
```

布局位于 `WindowLayout` 中，因此切换标签页时保持不变——切换标签页只改变该区域所有面板渲染的文档。

## 面板类型与模式迁移

### 树即真相

一个编辑器区域处于两种模式之一，由会话是否持有标签页推导（`editor_area_mode`）：

| 模式 | 树不变量 |
| --- | --- |
| `Welcome`（无标签页）| 所有面板都是 `Welcome(WelcomePanelKind)` |
| `Editing`（有标签页）| 所有面板都是 `Editing(EditingPanelKind)` |

渲染直接 match 面板类型——没有独立的模式分支：`Welcome(_)` 渲染欢迎提示，`Editing(k)` 渲染对应的编辑视图。

### 迁移

模式翻转时整棵树一起迁移，split 结构始终保留：

| 迁移 | 时机 | 效果 |
| --- | --- | --- |
| `enter_editing(area)` | 压入第一个标签页（`open_file_in_area`、`new_untitled_tab`、`from_markdown`）| `Welcome(None)` → `Editing(SourceCode)`；`Welcome(Some(k))` → `Editing(k)` |
| `exit_editing(area)` | 关闭最后一个标签页（`close_tab`）| `Editing(k)` → `Welcome(Some(k))` |

两者都是幂等的。因为欢迎面板**记住**它之前的编辑面板类型，关闭最后一个标签页再重新进入编辑时，之前的面板布局会被恢复：

```
Editing(Preview) ──关闭最后标签──▶ Welcome(Some(Preview)) ──打开文件──▶ Editing(Preview)
Editing(Wysiwyg) ────────────────▶ Welcome(Some(Wysiwyg)) ────────────▶ Editing(Wysiwyg)
(全新区域)      ─────────────────▶ Welcome(None) ──────────────────────▶ Editing(SourceCode)
```

实现通过收集叶子 id 并逐个 `set_leaf_kind` 改写类型（刻意避免"递归泛型 + `impl FnMut` 闭包"的写法——它会触发 rustc 1.97 的病态 codegen 性能问题）。

## 分屏操作

### 两个分屏入口

| 入口 | 函数 | 新面板类型 |
| --- | --- | --- |
| 状态栏分屏按钮（横向 / 纵向）| `split_editor_inner_panel` | 继承**聚焦**面板的类型，比例为 `0.5` |
| 面板四角拖动 | `split_editor_inner_panel_with_ratio` | 继承**被拖动**面板的类型，比例来自拖拽手势 |

继承是整类型继承，因此模式自动保持一致：欢迎面板分屏出欢迎面板（携带相同的记忆类型），编辑面板分屏出相同的编辑类型。如果无法解析目标类型（正常不应发生），新面板回退为 `Welcome(None)`。

欢迎状态下同样可以分屏，方便在打开文档前预先布置布局。

### 调整操作

| 操作 | 函数 |
| --- | --- |
| 拖动分割条 | `update_editor_inner_panel_splitter_drag` —— 仅改变比例 |
| 关闭面板 | `close_editor_inner_panel` —— 仅当剩余叶子数大于 1 时才移除 |
| 切换面板类型 | `change_editor_inner_panel_kind` —— 通过面板头部下拉菜单；参数是 `EditingPanelKind`，类型系统保证不会把面板切到欢迎模式 |

## 区域、会话与激活编辑器

- 编辑器区域切换到其他类型时，**仅当会话仍持有标签页**才保留会话（后台编辑）：切回 `Editor` 时恢复标签页与面板布局；空会话被丢弃（`change_window_area_kind`）。
- **激活编辑器**是最后被聚焦的*前台*编辑器（`active_editor_area` + `editor_activation_history`）。`is_foreground_editor` 是前台/后台区分的唯一消费者；explorer 打开文件作用于激活编辑器，前台没有编辑器时静默忽略——绝不会落入后台（保留的）会话。
- `EditorAreaMode`（`Welcome`/`Editing`）仍是面向渲染与状态栏的区域级查询；面板类型在面板层面编码同一事实。

## 聚焦设计

```rust
focused_editor_inner_panel: Option<InnerPanelLocation>   // { area_id, panel_id }
```

- **全局唯一**：整个窗口恰好有一个聚焦面板。
- **设置时机**：点击面板时（`on_mouse_down`）；首次渲染时自动聚焦第一个面板；关闭聚焦面板后，下一次渲染自动聚焦剩余的第一个。与编辑器区域交互也会激活该区域（上述激活规则）。
- **消费方**是匹配区域的状态栏（它通过 `area_id` 过滤全局聚焦）：
  - **模式胶囊** —— 始终可见：欢迎模式显示 `Welcome`（禁用）；编辑模式显示聚焦面板的类型（点击打开类型切换下拉菜单）。
  - **分屏按钮** —— 以聚焦面板为目标；新面板继承其类型。
  - **关闭按钮** —— 关闭聚焦面板（仅在多面板时显示）。

## 端到端流程

```
打开文件 / 双击欢迎提示   → 压入第一个标签页 → enter_editing：
                             Welcome(None) → Editing(SourceCode)
点击面板                  → focused = (area_id, panel_id)
状态栏（该区域）          → 模式胶囊显示聚焦面板类型
分屏按钮                  → 新面板继承聚焦类型，比例 0.5
四角拖动                  → 新面板继承被拖动类型，手势比例
所有面板渲染              → 该区域的激活标签页文档
切换标签页                → 面板树不变
关闭最后一个标签页        → exit_editing：面板变为
                             Welcome(Some(kind))，布局保留
区域切换到 Explorer       → 会话在仍有标签页时保留（后台编辑）；
                             空会话被丢弃
Explorer 打开文件         → 作用于激活（前台）编辑器；
                             没有前台编辑器时静默忽略
```

## 代码位置

| 关注点 | 文件 |
| --- | --- |
| 布局状态（区域、会话、聚焦、拖拽会话）| `src/layout/state.rs` |
| 分裂树操作 | `src/layout/tree.rs` |
| 面板类型与模式类型 | `src/layout/types.rs` |
| 内部面板渲染 | `src/editor/panels/layout/mod.rs` |
| 状态栏按钮与聚焦显示 | `src/windows/editor/status_bar.rs` |
