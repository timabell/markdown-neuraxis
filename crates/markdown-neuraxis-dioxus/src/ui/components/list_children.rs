use crate::ui::components::list_component::ListComponent;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{
    AnchorId, BlockKind, Cmd, Document, ListItem, Marker, RenderBlock,
};
use std::path::PathBuf;
use std::sync::Arc;

#[component]
pub fn ListChildren(
    item: ListItem,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Arc<Document>,
) -> Element {
    if item.children.is_empty() {
        return rsx! { {} };
    }

    let child_list_type = determine_child_list_type(&item.children);

    rsx! {
        ListComponent {
            items: item.children.clone(),
            list_type: child_list_type,
            notes_path,
            on_file_select,
            on_focus,
            on_command,
            on_wikilink_click,
            focused_anchor_id,
            document
        }
    }
}

// Determine the list type for children based on first child's marker
fn determine_child_list_type(children: &[ListItem]) -> &'static str {
    children
        .first()
        .and_then(|child| match &child.block.kind {
            BlockKind::ListItem {
                marker: Marker::Numbered(_),
                ..
            } => Some("ol"),
            _ => None,
        })
        .unwrap_or("ul")
}
