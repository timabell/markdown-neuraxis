use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::snapshot::{
    Block as EngineBlock, BlockContent, BlockKind, InlineSegment, SegmentKind,
};
use std::path::PathBuf;

/// Opens a URL in the system's default browser
fn open_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }

    Ok(())
}

/// Renders inline segments as formatted HTML elements
fn render_segments(segments: &[InlineSegment], on_wikilink_click: Callback<String>) -> Element {
    rsx! {
        for (i, segment) in segments.iter().enumerate() {
            {render_single_segment(segment.clone(), i, on_wikilink_click)}
        }
    }
}

/// Render a single InlineSegment
fn render_single_segment(
    segment: InlineSegment,
    key: usize,
    on_wikilink_click: Callback<String>,
) -> Element {
    match segment.kind {
        SegmentKind::Text(text) => rsx! {
            span { key: "{key}", "{text}" }
        },
        SegmentKind::Strong(text) => rsx! {
            strong { key: "{key}", "{text}" }
        },
        SegmentKind::Emphasis(text) => rsx! {
            em { key: "{key}", "{text}" }
        },
        SegmentKind::Code(text) => rsx! {
            code { key: "{key}", class: "inline-code", "{text}" }
        },
        SegmentKind::Strikethrough(text) => rsx! {
            del { key: "{key}", "{text}" }
        },
        SegmentKind::WikiLink { target, alias } => {
            let display_text = alias.unwrap_or_else(|| target.clone());
            let target_clone = target.clone();
            rsx! {
                a {
                    key: "{key}",
                    class: "wikilink",
                    href: "#",
                    onclick: move |evt: MouseEvent| {
                        evt.prevent_default();
                        evt.stop_propagation();
                        on_wikilink_click.call(target_clone.clone());
                    },
                    "{display_text}"
                }
            }
        }
        SegmentKind::Link { text, url } => {
            let url_clone = url.clone();
            rsx! {
                a {
                    key: "{key}",
                    class: "external-link",
                    href: "{url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    onclick: move |evt: MouseEvent| {
                        evt.prevent_default();
                        evt.stop_propagation();
                        if let Err(e) = open_url(&url_clone) {
                            eprintln!("Failed to open URL {}: {}", url_clone, e);
                        }
                    },
                    "{text}"
                }
            }
        }
        SegmentKind::Image { alt, url } => rsx! {
            img { key: "{key}", alt: "{alt}", src: "{url}" }
        },
        SegmentKind::HardBreak => rsx! {
            br { key: "{key}" }
        },
    }
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
            let segments = block.segments.clone();
            rsx! {
                div {
                    class: "{class}",
                    onclick: move |_| on_focus.call(()),
                    {render_segments(&segments, on_wikilink_click)}
                }
            }
        }
        BlockKind::Paragraph => {
            let segments = block.segments.clone();
            rsx! {
                p {
                    class: "paragraph",
                    onclick: move |_| on_focus.call(()),
                    {render_segments(&segments, on_wikilink_click)}
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
            let segments = block.segments.clone();
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
                        {render_segments(&segments, on_wikilink_click)}
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
            let segments = block.segments.clone();
            rsx! {
                pre {
                    class: "code-block {lang_class}",
                    onclick: move |_| on_focus.call(()),
                    code {
                        {render_segments(&segments, on_wikilink_click)}
                    }
                }
            }
        }
        BlockKind::BlockQuote => {
            let segments = block.segments.clone();
            rsx! {
                blockquote {
                    class: "blockquote",
                    onclick: move |_| on_focus.call(()),
                    {render_segments(&segments, on_wikilink_click)}
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
