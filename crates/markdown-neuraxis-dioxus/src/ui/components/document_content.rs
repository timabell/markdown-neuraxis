use crate::ui::components::content_group::ContentGroup;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, Snapshot};
use std::path::PathBuf;
use std::sync::Arc;

/// Component for document content rendering
#[component]
pub fn DocumentContent(
    snapshot: Snapshot,
    document: Arc<Document>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<Cmd>,
) -> Element {
    let grouped_content = &snapshot.content_groups;

    rsx! {
        div {
            class: "document-content",
            for (group_index, group) in grouped_content.iter().enumerate() {
                ContentGroup {
                    key: "{group_index}",
                    group: group.clone(),
                    group_index,
                    document: document.clone(),
                    focused_anchor_id,
                    on_file_select,
                    on_command
                }
            }
        }
    }
}
