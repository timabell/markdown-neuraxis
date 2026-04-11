use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::Cmd;

#[component]
pub fn EmptyDocument(
    on_command: Callback<Cmd>,
    #[props(default = true)] should_focus: bool,
) -> Element {
    let mut local_content = use_signal(String::new);

    // Helper to commit content if non-empty
    let mut commit_if_nonempty = {
        let on_command = on_command;
        let mut local_content = local_content;
        move || {
            let text = local_content.read().clone();
            if !text.trim().is_empty() {
                local_content.set(String::new()); // Clear to prevent double-commit
                on_command.call(Cmd::InsertText { at: 0, text });
            }
        }
    };

    rsx! {
        div {
            class: "empty-document",
            textarea {
                class: "editor-textarea",
                value: local_content.read().clone(),
                placeholder: "Start typing markdown...",
                spellcheck: false,
                rows: 2,

                onmounted: move |event: Event<MountedData>| async move {
                    if should_focus {
                        let _ = event.data().set_focus(true).await;
                    }
                },

                oninput: move |event: Event<FormData>| {
                    local_content.set(event.value());
                },

                onkeydown: move |event: Event<KeyboardData>| {
                    match event.key() {
                        Key::Enter if !event.modifiers().shift() => {
                            event.prevent_default();
                            commit_if_nonempty();
                        }
                        Key::Escape => {
                            commit_if_nonempty();
                        }
                        _ => {}
                    }
                },

                onblur: move |_| {
                    commit_if_nonempty();
                },
            }
        }
    }
}
