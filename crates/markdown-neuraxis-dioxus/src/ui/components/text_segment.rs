use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{InlineNode, InlineSegment};

/// Renders a list of InlineSegments
#[component]
pub fn InlineSegments(
    segments: Vec<InlineSegment>,
    on_wikilink_click: Callback<String>,
) -> Element {
    rsx! {
        for (i, segment) in segments.iter().enumerate() {
            {render_segment(segment.clone(), i, on_wikilink_click)}
        }
    }
}

/// Render a single InlineSegment
fn render_segment(
    segment: InlineSegment,
    key: usize,
    on_wikilink_click: Callback<String>,
) -> Element {
    render_inline_node(&segment.kind, key, on_wikilink_click)
}

/// Render an InlineNode (recursive for nested formatting)
fn render_inline_node(
    node: &InlineNode,
    key: usize,
    on_wikilink_click: Callback<String>,
) -> Element {
    match node {
        InlineNode::Text(text) => rsx! {
            span { key: "{key}", "{text}" }
        },
        InlineNode::Strong(children) => rsx! {
            strong { key: "{key}",
                for (i, child) in children.iter().enumerate() {
                    {render_inline_node(child, key * 1000 + i, on_wikilink_click)}
                }
            }
        },
        InlineNode::Emphasis(children) => rsx! {
            em { key: "{key}",
                for (i, child) in children.iter().enumerate() {
                    {render_inline_node(child, key * 1000 + i, on_wikilink_click)}
                }
            }
        },
        InlineNode::Code(text) => rsx! {
            code { key: "{key}", class: "inline-code", "{text}" }
        },
        InlineNode::Strikethrough(text) => rsx! {
            del { key: "{key}", "{text}" }
        },
        InlineNode::WikiLink { target, alias } => {
            let display_text = alias.clone().unwrap_or_else(|| target.clone());
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
        InlineNode::Link { text, url } => {
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
        InlineNode::Image { alt, url } => rsx! {
            img { key: "{key}", alt: "{alt}", src: "{url}" }
        },
        InlineNode::HardBreak => rsx! {
            br { key: "{key}" }
        },
        InlineNode::SoftBreak => rsx! {
            span { key: "{key}", " " }
        },
    }
}

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
