use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

/// Component for rendering raw HTML blocks in markdown.
/// HTML is valid markdown content, displayed as neutral monospace text.
#[component]
pub fn HtmlBlock(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        div {
            class: "html-block",
            tabindex: "0",
            onfocus: move |_| on_focus.call(()),
            onkeydown: move |evt| {
                if evt.key() == Key::Enter {
                    on_focus.call(());
                }
            },
            "{block.content}"
        }
    }
}
