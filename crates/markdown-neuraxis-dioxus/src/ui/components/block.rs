use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::snapshot::{Block as EngineBlock, BlockContent, BlockKind};
use std::path::PathBuf;

/// Extract content text from a block's line ranges
pub fn extract_content(block: &EngineBlock, source: &str) -> String {
    block
        .lines
        .iter()
        .map(|line| &source[line.content.clone()])
        .collect::<Vec<_>>()
        .join("\n")
}

#[component]
pub fn BlockRenderer(
    block: EngineBlock,
    source: String,
    depth: usize,
    notes_path: PathBuf,
    on_focus: Callback<()>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let content = extract_content(&block, &source);

    match &block.kind {
        BlockKind::Root => {
            // Render children
            if let BlockContent::Children(children) = &block.content {
                rsx! {
                    for (i, child) in children.iter().enumerate() {
                        BlockRenderer {
                            key: "{i}",
                            block: child.clone(),
                            source: source.clone(),
                            depth: depth,
                            notes_path: notes_path.clone(),
                            on_focus,
                            on_wikilink_click
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::Heading { level } => {
            let class = format!("heading heading-{}", level);
            rsx! {
                div {
                    class: "{class}",
                    onclick: move |_| on_focus.call(()),
                    "{content}"
                }
            }
        }
        BlockKind::Paragraph => {
            rsx! {
                p {
                    class: "paragraph",
                    onclick: move |_| on_focus.call(()),
                    "{content}"
                }
            }
        }
        BlockKind::List => {
            // Render list items as children
            if let BlockContent::Children(children) = &block.content {
                rsx! {
                    ul {
                        class: "list",
                        for (i, child) in children.iter().enumerate() {
                            BlockRenderer {
                                key: "{i}",
                                block: child.clone(),
                                source: source.clone(),
                                depth: depth + 1,
                                notes_path: notes_path.clone(),
                                on_focus,
                                on_wikilink_click
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::ListItem { marker } => {
            // Render list item with its content and nested children
            rsx! {
                li {
                    class: "list-item",
                    onclick: move |_| on_focus.call(()),
                    span {
                        class: "list-marker",
                        "{marker}"
                    }
                    span {
                        class: "list-content",
                        "{content}"
                    }
                    // Render nested content (paragraphs, nested lists)
                    if let BlockContent::Children(children) = &block.content {
                        for (i, child) in children.iter().enumerate() {
                            BlockRenderer {
                                key: "{i}",
                                block: child.clone(),
                                source: source.clone(),
                                depth: depth + 1,
                                notes_path: notes_path.clone(),
                                on_focus,
                                on_wikilink_click
                            }
                        }
                    }
                }
            }
        }
        BlockKind::FencedCode { language } => {
            let lang_class = language
                .as_ref()
                .map(|l| format!("language-{}", l))
                .unwrap_or_default();
            rsx! {
                pre {
                    class: "code-block {lang_class}",
                    onclick: move |_| on_focus.call(()),
                    code {
                        "{content}"
                    }
                }
            }
        }
        BlockKind::BlockQuote => {
            rsx! {
                blockquote {
                    class: "blockquote",
                    onclick: move |_| on_focus.call(()),
                    "{content}"
                }
            }
        }
        BlockKind::ThematicBreak => {
            rsx! {
                hr {
                    class: "thematic-break",
                    onclick: move |_| on_focus.call(())
                }
            }
        }
    }
}
