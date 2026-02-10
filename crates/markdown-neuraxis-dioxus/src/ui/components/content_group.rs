use crate::ui::components::blockquote_group::BlockquoteGroup;
use crate::ui::components::list_group::ListGroup;
use crate::ui::components::single_block::SingleBlock;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::ContentGroup as EditorContentGroup;
use std::path::PathBuf;
use std::sync::Arc;

/// Component for rendering individual content groups
#[component]
pub fn ContentGroup(
    group: EditorContentGroup,
    group_index: usize,
    notes_path: PathBuf,
    document: Arc<markdown_neuraxis_engine::editing::Document>,
    focused_anchor_id: Signal<Option<markdown_neuraxis_engine::editing::AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<markdown_neuraxis_engine::editing::Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    match group {
        EditorContentGroup::SingleBlock(block) => rsx! {
            SingleBlock {
                block,
                group_index,
                notes_path,
                document,
                focused_anchor_id,
                on_file_select,
                on_command,
                on_wikilink_click
            }
        },
        EditorContentGroup::BulletListGroup { .. }
        | EditorContentGroup::NumberedListGroup { .. } => rsx! {
            ListGroup {
                group,
                group_index,
                notes_path,
                document,
                focused_anchor_id,
                on_file_select,
                on_command,
                on_wikilink_click
            }
        },
        EditorContentGroup::BlockQuoteGroup { items } => rsx! {
            BlockquoteGroup {
                items,
                on_focus: move |_| {}
            }
        },
    }
}
