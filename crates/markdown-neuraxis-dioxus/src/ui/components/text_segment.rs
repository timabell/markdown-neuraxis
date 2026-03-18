use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{InlineSegment, SegmentKind};

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
