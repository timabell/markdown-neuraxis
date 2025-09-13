use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::snapshot::TextSegment;
use std::path::PathBuf;

/// Props for text segment rendering
#[derive(Props, Clone, PartialEq)]
pub struct TextSegmentProps {
    /// The text segments to render
    pub segments: Vec<TextSegment>,
    /// The notes directory path for resolving wikilinks
    pub notes_path: PathBuf,
    /// Callback for wikilink navigation
    pub on_wikilink_click: Callback<String>,
}

/// Renders a list of TextSegments with wikilink support
#[component]
pub fn TextSegments(props: TextSegmentProps) -> Element {
    let segments = props.segments;
    let on_wikilink_click = props.on_wikilink_click;

    rsx! {
        for segment in segments.into_iter() {
            {render_segment(segment, on_wikilink_click)}
        }
    }
}

/// Render a single TextSegment
fn render_segment(segment: TextSegment, on_wikilink_click: Callback<String>) -> Element {
    match segment {
        TextSegment::Text(text) => rsx! { span { "{text}" } },
        TextSegment::WikiLink { target } => {
            let target_clone = target.clone();
            rsx! {
                a {
                    class: "wikilink",
                    href: "#",
                    onclick: move |evt: MouseEvent| {
                        evt.prevent_default();
                        evt.stop_propagation();
                        on_wikilink_click.call(target_clone.clone());
                    },
                    "{target}"
                }
            }
        }
        TextSegment::Url { href } => {
            let href_clone = href.clone();
            rsx! {
                a {
                    class: "external-link",
                    href: "{href}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    onclick: move |evt: MouseEvent| {
                        evt.prevent_default();
                        evt.stop_propagation();
                        // Use system's default browser to open the URL
                        if let Err(e) = open_url(&href_clone) {
                            eprintln!("Failed to open URL {}: {}", href_clone, e);
                        }
                    },
                    "{href}",
                    span { class: "external-link-icon", " ↗" }
                }
            }
        }
    }
}

/// Renders content with wikilink support, falling back to plain text if no segments
#[component]
pub fn ContentWithWikiLinks(
    content: String,
    segments: Option<Vec<TextSegment>>,
    notes_path: PathBuf,
    on_wikilink_click: Callback<String>,
) -> Element {
    if let Some(segments) = segments {
        rsx! {
            TextSegments {
                segments,
                notes_path,
                on_wikilink_click
            }
        }
    } else {
        rsx! { span { "{content}" } }
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
