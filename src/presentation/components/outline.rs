use crate::domain::models::OutlineItem;
use dioxus::prelude::*;

#[component]
pub fn OutlineItemComponent(item: OutlineItem, indent: usize) -> Element {
    rsx! {
        div {
            class: "outline-item",
            style: "margin-left: {indent * 20}px;",
            "[{item.level}] {item.content}"
        }
        for child in &item.children {
            OutlineItemComponent { item: child.clone(), indent: indent + 1 }
        }
    }
}