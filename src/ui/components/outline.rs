use crate::models::{ContentBlock, ListItem};
use dioxus::prelude::*;

#[component]
pub fn OutlineItemComponent(
    item: ListItem,
    indent: usize,
    is_numbered: bool,
    item_number: Option<usize>,
) -> Element {
    rsx! {
        div {
            class: "list-item",
            style: "margin-left: {indent * 24}px;",
            div {
                class: "list-item-content",
                if is_numbered {
                    if let Some(num) = item_number {
                        span { class: "list-marker numbered", "{num}. " }
                    } else {
                        span { class: "list-marker numbered", "• " }
                    }
                } else {
                    span { class: "list-marker bullet", "• " }
                }
                span { class: "list-text", "{item.content}" }
            }
            // Render nested content (like code blocks)
            if !item.nested_content.is_empty() {
                div {
                    class: "nested-content",
                    style: "margin-left: {(indent + 1) * 24}px;",
                    for content in &item.nested_content {
                        NestedContentComponent { content: content.clone() }
                    }
                }
            }
        }
        if !item.children.is_empty() {
            div {
                class: "nested-list",
                for (idx, child) in item.children.iter().enumerate() {
                    OutlineItemComponent {
                        item: child.clone(),
                        indent: indent + 1,
                        is_numbered: is_numbered,
                        item_number: if is_numbered { Some(idx + 1) } else { None }
                    }
                }
            }
        }
    }
}

#[component]
fn NestedContentComponent(content: ContentBlock) -> Element {
    match content {
        ContentBlock::CodeBlock { language, code } => {
            let code_class = if let Some(ref lang) = language {
                format!("language-{lang}")
            } else {
                "language-text".to_string()
            };

            rsx! {
                div {
                    class: "code-block nested",
                    if let Some(lang) = language {
                        div { class: "code-language", "{lang}" }
                    }
                    pre {
                        code {
                            class: "{code_class}",
                            "{code}"
                        }
                    }
                }
            }
        }
        ContentBlock::Paragraph(text) => {
            rsx! {
                p { class: "paragraph nested", "{text}" }
            }
        }
        ContentBlock::Quote(text) => {
            rsx! {
                blockquote { class: "quote nested", "{text}" }
            }
        }
        ContentBlock::Rule => {
            rsx! {
                hr { class: "rule nested" }
            }
        }
        // For other content types, we don't expect them to be nested in list items
        // but handle them gracefully
        _ => rsx! {
            div { class: "unsupported-nested-content", "Unsupported nested content" }
        },
    }
}
