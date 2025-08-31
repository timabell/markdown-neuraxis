use crate::editing::{BlockKind, Marker, RenderBlock, Snapshot};
use crate::models::MarkdownFile;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn SnapshotMainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<()>,
) -> Element {
    let display_name = file.display_path();

    rsx! {
        div {
            class: "document-container",
            h1 { "📝 {display_name}" }
            hr {}
            if !snapshot.blocks.is_empty() {
                div {
                    class: "document-content",
                    for block in &snapshot.blocks {
                        RenderBlockComponent {
                            key: "{block.id:?}",
                            block: block.clone(),
                            on_file_select: on_file_select
                        }
                    }
                }
            } else {
                div {
                    class: "empty-document",
                    p { "This document appears to be empty." }
                    button {
                        class: "add-block-button",
                        onclick: move |_| {
                            // TODO: Implement add block functionality using editing core
                            // todo!("Add block functionality using editing core not yet implemented");
                        },
                        "Add first block +"
                    }
                }
            }
        }
    }
}

#[component]
pub fn RenderBlockComponent(
    block: RenderBlock,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    match block.kind {
        BlockKind::Heading { level } => {
            let class_name = format!("heading level-{level}");
            match level {
                1 => rsx! { h1 { class: "{class_name}", "{block.content}" } },
                2 => rsx! { h2 { class: "{class_name}", "{block.content}" } },
                3 => rsx! { h3 { class: "{class_name}", "{block.content}" } },
                4 => rsx! { h4 { class: "{class_name}", "{block.content}" } },
                5 => rsx! { h5 { class: "{class_name}", "{block.content}" } },
                _ => rsx! { h6 { class: "{class_name}", "{block.content}" } },
            }
        }
        BlockKind::Paragraph => {
            rsx! {
                p {
                    class: "paragraph",
                    "{block.content}"
                }
            }
        }
        BlockKind::ListItem { marker, depth } => {
            let marker_text = match marker {
                Marker::Dash => "-",
                Marker::Asterisk => "*",
                Marker::Plus => "+",
                Marker::Numbered => "1.", // TODO: Get actual number
            };
            let indent_style = format!("margin-left: {}px;", depth * 20);

            rsx! {
                div {
                    class: "list-item",
                    style: "{indent_style}",
                    span { class: "marker", "{marker_text} " }
                    span { class: "content", "{block.content}" }
                }
            }
        }
        BlockKind::CodeFence { lang } => {
            let code_class = if let Some(ref lang_str) = lang {
                format!("language-{lang_str}")
            } else {
                "language-text".to_string()
            };

            rsx! {
                div {
                    class: "code-block",
                    if let Some(lang_str) = lang {
                        div { class: "code-language", "{lang_str}" }
                    }
                    pre {
                        code {
                            class: "{code_class}",
                            "{block.content}"
                        }
                    }
                }
            }
        }
    }
}

/// EditorBlock component for raw markdown editing when a block is focused
/// This will implement the editing pattern from ADR-0004 where focused blocks
/// switch to raw markdown editing mode
#[component]
pub fn EditorBlock(
    block: RenderBlock,
    on_save: Callback<String>,
    on_cancel: Callback<()>,
) -> Element {
    // TODO: Implement raw markdown editing with controlled textarea
    // Following ADR-0004 pattern:
    // - Show exact bytes of content_range in textarea
    // - Use beforeinput -> preventDefault -> send Cmd -> apply() -> update value
    // - Handle Tab/Shift+Tab for indent/outdent
    // - Handle Enter for split list item
    // - Align textarea with gutter for indent/marker
    todo!("EditorBlock component not yet implemented - will provide raw markdown editing")
}
