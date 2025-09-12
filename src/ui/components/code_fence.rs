use crate::editing::RenderBlock;
use dioxus::prelude::*;

#[component]
pub fn CodeFence(block: RenderBlock, lang: Option<String>, on_focus: Callback<()>) -> Element {
    let code_class = lang
        .as_ref()
        .map(|l| format!("language-{l}"))
        .unwrap_or_else(|| "language-text".to_string());

    rsx! {
        div {
            class: "code-block clickable-block",
            onclick: move |_| on_focus.call(()),
            if let Some(lang_str) = lang {
                div { class: "code-language", "{lang_str}" }
            }
            pre {
                code {
                    class: "{code_class}",
                    "{block.content}"
                }
            }
        }
    }
}
