use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::snapshot::BlockQuoteItem;

/// Component for rendering nested blockquotes
#[component]
pub fn BlockquoteGroup(items: Vec<BlockQuoteItem>, on_focus: Callback<()>) -> Element {
    rsx! {
        for item in items {
            BlockquoteItem {
                item,
                on_focus
            }
        }
    }
}

/// Renders a single blockquote item with nested children
#[component]
fn BlockquoteItem(item: BlockQuoteItem, on_focus: Callback<()>) -> Element {
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
            "{item.block.content}"
            // Render nested children inside this blockquote
            for child in item.children {
                BlockquoteItem {
                    item: child,
                    on_focus
                }
            }
        }
    }
}
