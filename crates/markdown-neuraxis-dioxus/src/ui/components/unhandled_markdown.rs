use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

/// Component for rendering unhandled markdown content (horizontal rules, etc.)
/// This component acts as a fallback for markdown elements we don't have specific handling for.
/// It renders the content as-is, similar to how Paragraph works.
#[component]
pub fn UnhandledMarkdown(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        div {
            class: "unhandled-markdown",
            tabindex: "0",
            onfocus: move |_| on_focus.call(()),
            onkeydown: move |evt| {
                if evt.key() == dioxus::events::Key::Enter {
                    on_focus.call(());
                }
            },
            // Render the raw content as-is
            "{block.content}"
        }
    }
}
