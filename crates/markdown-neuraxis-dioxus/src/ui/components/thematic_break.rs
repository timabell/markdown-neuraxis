use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

/// Component for rendering thematic breaks (horizontal rules)
/// Renders as a clickable/editable <hr> element
#[component]
pub fn ThematicBreak(block: RenderBlock, on_focus: Callback<()>) -> Element {
    rsx! {
        hr {
            class: "thematic-break",
            tabindex: "0",
            onfocus: move |_| on_focus.call(()),
            onkeydown: move |evt| {
                if evt.key() == dioxus::events::Key::Enter {
                    on_focus.call(());
                }
            }
        }
    }
}
