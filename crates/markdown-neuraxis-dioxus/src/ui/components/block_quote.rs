use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

/// Component for rendering blockquotes (> quoted text)
/// Renders as a clickable/editable <blockquote> element
#[component]
pub fn BlockQuote(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        blockquote {
            class: "block-quote",
            tabindex: "0",
            onfocus: move |_| on_focus.call(()),
            onkeydown: move |evt| {
                if evt.key() == dioxus::events::Key::Enter {
                    on_focus.call(());
                }
            },
            // Render the content (which includes the > markers)
            "{block.content}"
        }
    }
}
