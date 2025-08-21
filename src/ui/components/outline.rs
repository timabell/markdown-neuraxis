use crate::models::{ContentBlock, ListItem, TextSegment};
use dioxus::prelude::*;
use std::path::{Path, PathBuf};

/// Resolve a wiki-link target to a file path (may or may not exist)
fn resolve_wiki_link(target: &str, notes_path: &Path) -> PathBuf {
    // Handle different wiki-link formats:
    // 1. Simple page name: "Getting-Started" -> Getting-Started.md
    // 2. Path with folders: "1_Projects/Project-Alpha" -> 1_Projects/Project-Alpha.md
    // 3. Path ending with .md: "some/page.md" -> some/page.md
    // 4. Journal entries: "journal/2024-01-15" -> journal/2024-01-15.md

    if target.ends_with(".md") {
        notes_path.join(target)
    } else {
        notes_path.join(format!("{target}.md"))
    }
}

#[component]
pub fn OutlineItemComponent(
    item: ListItem,
    indent: usize,
    is_numbered: bool,
    item_number: Option<usize>,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
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
                span {
                    class: "list-text",
                    if let Some(ref segments) = item.segments {
                        for segment in segments {
                            TextSegmentComponent { segment: segment.clone(), notes_path: notes_path.clone(), on_file_select: on_file_select }
                        }
                    } else {
                        "{item.content}"
                    }
                }
            }
            // Render nested content (like code blocks)
            if !item.nested_content.is_empty() {
                div {
                    class: "nested-content",
                    style: "margin-left: {(indent + 1) * 24}px;",
                    for content in &item.nested_content {
                        NestedContentComponent { content: content.clone(), notes_path: notes_path.clone(), on_file_select: on_file_select }
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
                        item_number: if is_numbered { Some(idx + 1) } else { None },
                        notes_path: notes_path.clone(),
                        on_file_select: on_file_select
                    }
                }
            }
        }
    }
}

#[component]
fn NestedContentComponent(
    content: ContentBlock,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
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
        ContentBlock::Paragraph { segments } => {
            rsx! {
                p {
                    class: "paragraph nested",
                    for segment in segments {
                        TextSegmentComponent { segment: segment.clone(), notes_path: notes_path.clone(), on_file_select: on_file_select }
                    }
                }
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

#[component]
pub fn TextSegmentComponent(
    segment: TextSegment,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    match segment {
        TextSegment::Text(text) => {
            // Handle hard breaks (trailing spaces + newline from markdown)
            if text.contains("  \n") {
                // Split on the original pattern and render with <br> tags
                let parts: Vec<&str> = text.split("  \n").collect();
                rsx! {
                    span {
                        for (i, part) in parts.iter().enumerate() {
                            "{part}"
                            if i < parts.len() - 1 {
                                br {}
                            }
                        }
                    }
                }
            } else {
                rsx! { "{text}" }
            }
        }
        TextSegment::WikiLink { target } => {
            rsx! {
                a {
                    href: "#",
                    class: "wiki-link",
                    "data-target": "{target}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        evt.stop_propagation(); // Stop the event from bubbling up to the editable block
                        if let Some(callback) = on_file_select {
                            let file_path = resolve_wiki_link(&target, &notes_path);
                            callback.call(file_path);
                        }
                    },
                    "{target}"
                }
            }
        }
    }
}
