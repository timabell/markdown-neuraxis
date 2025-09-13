use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;

#[component]
pub fn Heading(block: RenderBlock, level: u32, on_focus: Callback<()>) -> Element {
    let class_name = format!("heading level-{level} clickable-block");
    let content = block.content.clone();

    match level {
        1 => {
            rsx! { h1 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
        2 => {
            rsx! { h2 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
        3 => {
            rsx! { h3 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
        4 => {
            rsx! { h4 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
        5 => {
            rsx! { h5 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
        _ => {
            rsx! { h6 { class: "{class_name}", onclick: move |_| on_focus.call(()), "{content}" } }
        }
    }
}
