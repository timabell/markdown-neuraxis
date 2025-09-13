use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

#[component]
pub fn Paragraph(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        p {
            class: "paragraph clickable-block",
            onclick: move |_| on_focus.call(()),
            "{block.content}"
        }
    }
}
