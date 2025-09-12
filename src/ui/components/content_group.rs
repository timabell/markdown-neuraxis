use crate::editing::ContentGroup as EditorContentGroup;
use crate::ui::components::list_group::ListGroup;
use crate::ui::components::single_block::SingleBlock;
use dioxus::prelude::*;
use std::path::PathBuf;

/// Component for rendering individual content groups
#[component]
pub fn ContentGroup(
    group: EditorContentGroup,
    group_index: usize,
    document: crate::editing::Document,
    focused_anchor_id: Signal<Option<crate::editing::AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<crate::editing::Cmd>,
) -> Element {
    match group {
        EditorContentGroup::SingleBlock(block) => rsx! {
            SingleBlock {
                block,
                group_index,
                document,
                focused_anchor_id,
                on_file_select,
                on_command
            }
        },
        EditorContentGroup::BulletListGroup { .. }
        | EditorContentGroup::NumberedListGroup { .. } => rsx! {
            ListGroup {
                group,
                group_index,
                document,
                focused_anchor_id,
                on_file_select,
                on_command
            }
        },
    }
}
