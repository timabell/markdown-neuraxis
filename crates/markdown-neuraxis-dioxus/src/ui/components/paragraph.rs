use crate::ui::components::text_segment::ContentWithWikiLinks;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;
use std::path::PathBuf;

#[component]
pub fn Paragraph(
    block: RenderBlock,
    notes_path: PathBuf,
    on_focus: Callback<()>,
    on_wikilink_click: Callback<String>,
) -> Element {
    rsx! {
        p {
            class: "paragraph clickable-block",
            onclick: move |_| on_focus.call(()),
            ContentWithWikiLinks {
                content: block.content.clone(),
                segments: block.segments.clone(),
                notes_path,
                on_wikilink_click
            }
        }
    }
}
