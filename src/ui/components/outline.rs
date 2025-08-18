use crate::models::ListItem;
use dioxus::prelude::*;

#[component]
pub fn OutlineItemComponent(
    item: ListItem,
    indent: usize,
    is_numbered: bool,
    item_number: Option<usize>,
) -> Element {
    rsx! {
        div {
            class: "list-item",
            style: "margin-left: {indent * 24}px;",
            div {
                class: "list-item-content",
                if is_numbered {
                    if let Some(num) = item_number {
                        span { class: "list-marker numbered", "{num}. " }
                    } else {
                        span { class: "list-marker numbered", "• " }
                    }
                } else {
                    span { class: "list-marker bullet", "• " }
                }
                span { class: "list-text", "{item.content}" }
            }
        }
        if !item.children.is_empty() {
            div {
                class: "nested-list",
                for (idx, child) in item.children.iter().enumerate() {
                    OutlineItemComponent {
                        item: child.clone(),
                        indent: indent + 1,
                        is_numbered: is_numbered,
                        item_number: if is_numbered { Some(idx + 1) } else { None }
                    }
                }
            }
        }
    }
}
