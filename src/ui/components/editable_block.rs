use crate::models::{BlockId, ContentBlock};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn EditableBlock(
    block: ContentBlock,
    block_id: BlockId,
    editing_raw: Option<String>, // Some(raw) if this block is being edited
    is_selected: bool,           // Whether this block is selected for navigation
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
    on_editing_end: Option<Callback<()>>, // Called when editing ends to restore document focus
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    if let Some(raw) = editing_raw {
        let mut content = use_signal(|| raw.clone());

        let save_content = move || {
            on_save.call((block_id, content.read().clone()));
            // Request document focus after editing ends
            if let Some(focus_callback) = on_editing_end {
                focus_callback.call(());
            }
        };

        rsx! {
            textarea {
                class: "block-editor",
                value: content.read().clone(),
                autofocus: true,
                rows: content.read().lines().count().max(3),
                onmounted: move |evt| {
                    // Force focus when textarea is mounted
                    spawn(async move {
                        let _ = evt.set_focus(true).await;
                    });
                },
                oninput: move |evt| {
                    content.set(evt.value());
                },
                onblur: move |_| {
                    save_content();
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape || (evt.key() == Key::Enter && evt.data().modifiers().ctrl()) {
                        save_content();
                    }
                }
            }
        }
    } else {
        let block_class = if is_selected {
            "editable-block selected"
        } else {
            "editable-block"
        };

        rsx! {
            div {
                class: "{block_class}",
                onclick: move |_| on_edit.call(block_id),
                // Render the block normally using existing components
                super::ContentBlockComponent {
                    block: block,
                    on_file_select: on_file_select
                }
            }
        }
    }
}
