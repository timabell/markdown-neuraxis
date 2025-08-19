use crate::models::{ContentBlock, Document};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn MainPanel(
    file: PathBuf,
    notes_path: PathBuf,
    document: Document,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    let display_name = if let Ok(relative) = file.strip_prefix(&notes_path) {
        relative.to_string_lossy().to_string()
    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        "Selected File".to_string()
    };

    rsx! {
        h1 { "📝 {display_name}" }
        hr {}
        if !document.content.is_empty() {
            div {
                class: "document-content",
                for block in &document.content {
                    ContentBlockComponent { block: block.clone(), notes_path: notes_path.clone(), on_file_select: on_file_select }
                }
            }
        } else {
            div {
                class: "empty-document",
                p { "This document appears to be empty." }
            }
        }
    }
}

#[component]
fn ContentBlockComponent(
    block: ContentBlock,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    match block {
        ContentBlock::Heading { level, text } => {
            let class_name = format!("heading level-{level}");
            match level {
                1 => rsx! { h1 { class: "{class_name}", "{text}" } },
                2 => rsx! { h2 { class: "{class_name}", "{text}" } },
                3 => rsx! { h3 { class: "{class_name}", "{text}" } },
                4 => rsx! { h4 { class: "{class_name}", "{text}" } },
                5 => rsx! { h5 { class: "{class_name}", "{text}" } },
                _ => rsx! { h6 { class: "{class_name}", "{text}" } },
            }
        }
        ContentBlock::Paragraph { segments } => {
            rsx! {
                p {
                    class: "paragraph",
                    for segment in segments {
                        super::TextSegmentComponent { segment: segment.clone(), notes_path: notes_path.clone(), on_file_select: on_file_select }
                    }
                }
            }
        }
        ContentBlock::BulletList { items } => {
            rsx! {
                div {
                    class: "bullet-list",
                    for item in items {
                        super::OutlineItemComponent {
                            item: item.clone(),
                            indent: 0,
                            is_numbered: false,
                            item_number: None,
                            notes_path: notes_path.clone(),
                            on_file_select: on_file_select
                        }
                    }
                }
            }
        }
        ContentBlock::NumberedList { items } => {
            rsx! {
                div {
                    class: "numbered-list",
                    for (idx, item) in items.iter().enumerate() {
                        super::OutlineItemComponent {
                            item: item.clone(),
                            indent: 0,
                            is_numbered: true,
                            item_number: Some(idx + 1),
                            notes_path: notes_path.clone(),
                            on_file_select: on_file_select
                        }
                    }
                }
            }
        }
        ContentBlock::CodeBlock { language, code } => {
            let code_class = if let Some(ref lang) = language {
                format!("language-{lang}")
            } else {
                "language-text".to_string()
            };

            rsx! {
                div {
                    class: "code-block",
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
        ContentBlock::Quote(text) => {
            rsx! {
                blockquote { class: "quote", "{text}" }
            }
        }
        ContentBlock::Rule => {
            rsx! {
                hr { class: "rule" }
            }
        }
    }
}
