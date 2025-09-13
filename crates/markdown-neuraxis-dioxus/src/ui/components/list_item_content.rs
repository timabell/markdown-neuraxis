use crate::ui::components::editor_block::EditorBlock;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, ListItem, RenderBlock};
use std::sync::Arc;

#[component]
pub fn ListItemContent(
    item: ListItem,
    is_focused: bool,
    document: Arc<Document>,
    on_command: Callback<Cmd>,
    on_focus: Callback<RenderBlock>,
    focused_anchor_id: Signal<Option<AnchorId>>,
) -> Element {
    if is_focused {
        rsx! {
            EditorBlock {
                block: item.block.clone(),
                content_text: item.block.content.clone(),
                on_command,
                on_cancel: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |_| focused_anchor_id.set(None)
                }
            }
        }
    } else {
        let block = item.block.clone();
        rsx! {
            span {
                class: "list-content clickable-block",
                onclick: move |evt: MouseEvent| {
                    evt.stop_propagation();
                    on_focus.call(block.clone());
                },
                "{item.block.content}"
            }
        }
    }
}
