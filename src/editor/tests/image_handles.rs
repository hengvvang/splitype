//! Image handle installation and reference resolution across
//! lists, quotes, callouts, and table cells.

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;
use crate::model::block::image::{
    ImageReferenceDefinitions, ImageResolvedSource, TableCellInlineImageSegment,
    parse_table_cell_inline_images,
};
use crate::model::parse::BlockKind;
use std::path::PathBuf;

#[gpui::test]
async fn standalone_root_image_installs_handle_and_resolves_relative_path(cx: &mut TestAppContext) {
    let markdown = "![diagram](./assets/diagram.png \"System diagram\")".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.title.as_deref(), Some("System diagram"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn standalone_root_image_with_underscores_installs_handle(cx: &mut TestAppContext) {
    let markdown =
        "![1.1_进制转换例子](./NetworkEngineerSummer.assets/1.1_进制转换例子.jpg)".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "1.1_进制转换例子");
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("NetworkEngineerSummer.assets/1.1_进制转换例子.jpg")
            )
        );
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn indented_root_images_install_handle_before_indented_code(cx: &mut TestAppContext) {
    let url1 = "https://gitee.com/jikeyang/typera_picgo/raw/master/sias/202508201435626.png";
    let url2 = "https://gitee.com/jikeyang/typera_picgo/raw/master/sias/202508201438742.png";
    let url3 = "https://gitee.com/jikeyang/typera_picgo/raw/master/sias/202508201439288.png";
    let url4 = "https://gitee.com/jikeyang/typera_picgo/raw/master/sias/202508201419865.png";
    let markdown = [
        format!("![image-1]({})", url1.replace("_", "\\_")),
        String::new(),
        format!("   ![image-2]({})", url2.replace("_", "\\_")),
        String::new(),
        format!("        ![image-3]({})", url3.replace("_", "\\_")),
        String::new(),
        "   所有组或用户名均对**Anaconda安装目录**的权限设置为**完全控制**后，如下图所示："
            .to_string(),
        String::new(),
        format!("![image-4]({})", url4.replace("_", "\\_")),
        String::new(),
        "    plain indented code".to_string(),
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        let image_sources = roots
            .iter()
            .filter_map(|block| {
                block
                    .read(cx)
                    .image_handle()
                    .map(|handle| handle.src.clone())
            })
            .collect::<Vec<_>>();
        assert_eq!(image_sources, vec![url1, url2, url3, url4]);
        assert!(
            roots
                .iter()
                .any(|block| matches!(block.read(cx).kind(), BlockKind::CodeBlock { .. }))
        );
    });
}

#[gpui::test]
async fn mixed_text_does_not_activate_image_handle(cx: &mut TestAppContext) {
    let markdown = "before ![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        assert!(block.read(cx).image_handle().is_none());
    });
}

#[gpui::test]
async fn reference_style_root_image_installs_handle(cx: &mut TestAppContext) {
    let markdown =
        "![reference image][ref-image]\n\n[ref-image]: ./assets/ref-image.png \"Caption\""
            .to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "reference image");
        assert_eq!(handle.src, "./assets/ref-image.png");
        assert_eq!(handle.title.as_deref(), Some("Caption"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/ref-image.png")
            )
        );
    });
}

#[gpui::test]
async fn quote_child_standalone_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = ">     ![diagram](./assets/diagram.png \"Caption\")".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("quote root").clone();
        let image_block = quote
            .read(cx)
            .children
            .first()
            .expect("quote image child")
            .clone();
        let handle = image_block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Caption"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn bulleted_list_item_standalone_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = "-     ![diagram](./assets/diagram.png \"Caption\")".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("list item root").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Caption"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn html_fallback_before_image_does_not_swallow_standalone_image(cx: &mut TestAppContext) {
    let image_url = "https://gitee.com/jikeyang/typera_picgo/raw/master/sias/202508200941158.png";
    let markdown = format!(
        "<span style='color:blue;'>Anaconda下载地址</span>：https://mirrors.tuna.tsinghua.edu.cn/anaconda/archive/\n\n![image-20250820094109009]({image_url})"
    );
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.doc().root_count(), 2);
        {
            let html = editor.doc().root_blocks()[0].read(cx);
            assert_eq!(html.kind(), BlockKind::HtmlBlock);
            assert!(
                html.display_text()
                    .starts_with("<span style='color:blue;'>")
            );
            assert!(
                html.data
                    .html
                    .as_ref()
                    .is_some_and(|html| html.is_semantic())
            );
        }

        let image = editor.doc().root_blocks()[1].read(cx);
        let handle = image.image_handle().expect("image handle");
        assert_eq!(handle.alt, "image-20250820094109009");
        assert_eq!(handle.src, image_url);
        match &handle.resolved_source {
            ImageResolvedSource::Remote(uri) => assert_eq!(uri.to_string(), image_url),
            other => panic!("expected remote image, got {other:?}"),
        }
    });
}

#[gpui::test]
async fn unclosed_html_fallback_stops_before_standalone_image_without_blank(
    cx: &mut TestAppContext,
) {
    let image_url = "https://example.com/image.png";
    let markdown = format!("<span>unclosed html\n![image]({image_url})");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.doc().root_count(), 2);
        assert_eq!(
            editor.doc().root_blocks()[0].read(cx).kind(),
            BlockKind::RawMarkdown
        );
        let image = editor.doc().root_blocks()[1].read(cx);
        let handle = image.image_handle().expect("image handle");
        assert_eq!(handle.alt, "image");
        assert_eq!(handle.src, image_url);
    });
}

#[gpui::test]
async fn numbered_list_item_standalone_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = "1. ![diagram](https://example.com/diagram.gif \"Caption\")".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("list item root").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.title.as_deref(), Some("Caption"));
        match &handle.resolved_source {
            ImageResolvedSource::Remote(uri) => {
                assert_eq!(uri.to_string(), "https://example.com/diagram.gif");
            }
            other => panic!("expected remote source, got {other:?}"),
        }
    });
}

#[gpui::test]
async fn task_list_item_reference_style_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = "- [ ] ![diagram][cover]\n\n[cover]: ./assets/diagram.png \"Cover\"".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let block = editor
            .doc()
            .first_root()
            .expect("task list item root")
            .clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Cover"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn mixed_list_item_title_does_not_activate_image_handle(cx: &mut TestAppContext) {
    let markdown = "- text ![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("list item root").clone();
        assert!(block.read(cx).image_handle().is_none());
    });
}

#[gpui::test]
async fn list_child_reference_style_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = [
        "- item",
        "  ![diagram][cover]",
        "",
        "[cover]: ./assets/diagram.png \"Cover\"",
    ]
    .join("\n");
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let list_item = editor.doc().first_root().expect("list item root").clone();
        let image_block = list_item
            .read(cx)
            .children
            .first()
            .expect("list child image")
            .clone();
        let handle = image_block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Cover"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn list_scoped_reference_definition_supports_list_item_image_handle(cx: &mut TestAppContext) {
    let markdown = [
        "- ![diagram][cover]",
        "  [cover]: ./assets/diagram.png \"Cover\"",
    ]
    .join("\n");
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let list_item = editor.doc().first_root().expect("list item root").clone();
        let handle = list_item.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Cover"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
        assert_eq!(
            list_item
                .read(cx)
                .children
                .first()
                .expect("reference definition child")
                .read(cx)
                .kind(),
            BlockKind::RawMarkdown
        );
    });
}

#[gpui::test]
async fn quote_list_item_standalone_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = "> - ![diagram](./assets/diagram.png)".to_string();
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("quote root").clone();
        let list_item = quote
            .read(cx)
            .children
            .first()
            .expect("quote list child")
            .clone();
        let handle = list_item.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn callout_task_list_reference_style_image_uses_container_scoped_definition(
    cx: &mut TestAppContext,
) {
    let markdown = [
        "> [!NOTE]",
        "> - [ ] ![diagram][cover]",
        ">",
        "> [cover]: ./assets/diagram.png \"Cover\"",
    ]
    .join("\n");
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        let list_item = callout
            .read(cx)
            .children
            .first()
            .expect("callout list child")
            .clone();
        let handle = list_item.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Cover"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn callout_list_child_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = [
        "> [!NOTE]",
        "> - item",
        ">   ![diagram](./assets/diagram.png)",
    ]
    .join("\n");
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        let list_item = callout
            .read(cx)
            .children
            .first()
            .expect("callout list child")
            .clone();
        let image_block = list_item
            .read(cx)
            .children
            .first()
            .expect("list child image")
            .clone();
        let handle = image_block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn callout_child_reference_style_image_uses_container_scoped_definition(
    cx: &mut TestAppContext,
) {
    let markdown = [
        "> [!NOTE]",
        ">     ![diagram][anim]",
        ">",
        "> [anim]: ./assets/diagram.png \"Animated\"",
    ]
    .join("\n");
    let file_path = PathBuf::from("D:/explorer/docs/note.md");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, Some(file_path.clone())));

    editor.read_with(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        let image_block = callout
            .read(cx)
            .children
            .iter()
            .find(|child| {
                child.read(cx).kind() == BlockKind::Paragraph
                    && child.read(cx).image_handle().is_some()
            })
            .expect("callout image child")
            .clone();
        let handle = image_block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "diagram");
        assert_eq!(handle.src, "./assets/diagram.png");
        assert_eq!(handle.title.as_deref(), Some("Animated"));
        assert_eq!(
            handle.resolved_source,
            ImageResolvedSource::Local(
                file_path
                    .parent()
                    .expect("file parent")
                    .join("assets/diagram.png")
            )
        );
    });
}

#[gpui::test]
async fn table_cell_with_standalone_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = [
        "| Preview |",
        "| --- |",
        "|    ![diagram](https://example.com/diagram.gif \"Animated\") |",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let handle = table.read(cx).table_grid.as_ref().expect("table grid");
        let cell_runtime = handle.rows[0][0]
            .read(cx)
            .image_handle()
            .expect("cell image handle");
        assert_eq!(cell_runtime.alt, "diagram");
        assert_eq!(cell_runtime.title.as_deref(), Some("Animated"));
        match &cell_runtime.resolved_source {
            ImageResolvedSource::Remote(uri) => {
                assert_eq!(uri.to_string(), "https://example.com/diagram.gif");
            }
            other => panic!("expected remote source, got {other:?}"),
        }
    });
}

#[gpui::test]
async fn table_cell_with_mixed_inline_image_uses_inline_image_segments(cx: &mut TestAppContext) {
    let markdown = [
        "| Preview |",
        "| --- |",
        "| image ![alt](https://example.com/x.png) |",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let handle = table.read(cx).table_grid.as_ref().expect("table grid");
        let cell = handle.rows[0][0].read(cx);
        assert!(cell.image_handle().is_none());

        let segments = parse_table_cell_inline_images(&cell.data.text_markdown());
        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            TableCellInlineImageSegment::Text("image ".to_string())
        );
        assert!(matches!(
            &segments[1],
            TableCellInlineImageSegment::Image { syntax, .. }
                if syntax.alt == "alt"
                    && syntax
                        .resolve_target(&ImageReferenceDefinitions::default())
                        .is_some_and(|target| target.src == "https://example.com/x.png")
        ));
    });
}

#[gpui::test]
async fn table_cell_with_reference_style_image_installs_handle(cx: &mut TestAppContext) {
    let markdown = [
        "| Preview |",
        "| --- |",
        "| ![diagram][anim] |",
        "",
        "[anim]: https://example.com/diagram.gif \"Animated\"",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let handle = table.read(cx).table_grid.as_ref().expect("table grid");
        let cell_runtime = handle.rows[0][0]
            .read(cx)
            .image_handle()
            .expect("cell image handle");
        assert_eq!(cell_runtime.alt, "diagram");
        assert_eq!(cell_runtime.title.as_deref(), Some("Animated"));
        match &cell_runtime.resolved_source {
            ImageResolvedSource::Remote(uri) => {
                assert_eq!(uri.to_string(), "https://example.com/diagram.gif");
            }
            other => panic!("expected remote source, got {other:?}"),
        }
    });
}

#[gpui::test]
async fn reference_style_link_in_root_paragraph_resolves_document_wide(cx: &mut TestAppContext) {
    let markdown = [
        "[reference link][ref-link]",
        "",
        "[ref-link]: https://example.com",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        assert_eq!(block.read(cx).display_text(), "reference link");
        assert_eq!(
            block.read(cx).inline_link_at(0),
            Some("https://example.com")
        );
    });
}

#[gpui::test]
async fn reference_style_link_in_table_cell_resolves_document_wide(cx: &mut TestAppContext) {
    let markdown = [
        "| Link |",
        "| --- |",
        "| [reference link][ref-link] |",
        "",
        "[ref-link]: https://example.com",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let handle = table.read(cx).table_grid.as_ref().expect("table grid");
        let cell = handle.rows[0][0].clone();
        assert_eq!(cell.read(cx).display_text(), "reference link");
        assert_eq!(cell.read(cx).inline_link_at(0), Some("https://example.com"));
    });
}
