use crate::ui::components::text_segment::ContentWithWikiLinks;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;
use std::path::PathBuf;

#[component]
pub fn Heading(
    block: RenderBlock,
    level: u32,
    notes_path: PathBuf,
    on_focus: Callback<()>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let class_name = format!("heading level-{level} clickable-block");

    let content_element = rsx! {
        ContentWithWikiLinks {
            content: block.content.clone(),
            segments: block.segments.clone(),
            notes_path,
            on_wikilink_click
        }
    };

    match level {
        1 => {
            rsx! { h1 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
        2 => {
            rsx! { h2 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
        3 => {
            rsx! { h3 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
        4 => {
            rsx! { h4 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
        5 => {
            rsx! { h5 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
        _ => {
            rsx! { h6 { class: "{class_name}", onclick: move |_| on_focus.call(()), {content_element} } }
        }
    }
}
