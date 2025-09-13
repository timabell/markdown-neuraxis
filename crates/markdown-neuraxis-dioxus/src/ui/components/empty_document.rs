use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::Cmd;

#[component]
pub fn EmptyDocument(on_command: Callback<Cmd>) -> Element {
    rsx! {
        div {
            class: "empty-document",
            p { "This document appears to be empty." }
            button {
                class: "add-block-button",
                onclick: move |_| {
                    let cmd = Cmd::InsertText {
                        at: 0,
                        text: "- ".to_string(),
                    };
                    on_command.call(cmd);
                },
                "Add first block +"
            }
        }
    }
}
