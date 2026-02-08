use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

/// Fallback component for single blockquotes (nested ones use BlockquoteGroup)
#[component]
pub fn BlockQuote(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        blockquote {
            class: "block-quote",
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
