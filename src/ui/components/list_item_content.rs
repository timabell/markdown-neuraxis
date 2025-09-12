use crate::editing::{AnchorId, Cmd, Document, ListItem, RenderBlock};
use crate::ui::components::editor_block::EditorBlock;
use dioxus::prelude::*;

#[component]
pub fn ListItemContent(
    item: ListItem,
    is_focused: bool,
    document: Document,
    on_command: Callback<Cmd>,
    on_focus: Callback<RenderBlock>,
    focused_anchor_id: Signal<Option<AnchorId>>,
) -> Element {
    if is_focused {
        rsx! {
            EditorBlock {
                block: item.block.clone(),
                content_text: document.slice_to_cow(item.block.byte_range.clone()).to_string(),
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
