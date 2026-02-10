use crate::ui::components::list_component::ListComponent;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::ContentGroup as EditorContentGroup;
use std::path::PathBuf;
use std::sync::Arc;

/// Component to render a list group
#[component]
pub fn ListGroup(
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
        EditorContentGroup::BulletListGroup { items } => {
            rsx! {
                ListComponent {
                    key: "{group_index}-bullet-list",
                    items,
                    list_type: "ul",
                    notes_path: notes_path.clone(),
                    on_file_select,
                    on_focus: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |block: markdown_neuraxis_engine::editing::RenderBlock| {
                            focused_anchor_id.set(Some(block.id));
                        }
                    },
                    on_command,
                    on_wikilink_click,
                    focused_anchor_id,
                    document
                }
            }
        }
        EditorContentGroup::NumberedListGroup { items } => {
            rsx! {
                ListComponent {
                    key: "{group_index}-numbered-list",
                    items,
                    list_type: "ol",
                    notes_path,
                    on_file_select,
                    on_focus: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |block: markdown_neuraxis_engine::editing::RenderBlock| {
                            focused_anchor_id.set(Some(block.id));
                        }
                    },
                    on_command,
                    on_wikilink_click,
                    focused_anchor_id,
                    document
                }
            }
        }
        EditorContentGroup::SingleBlock(_) | EditorContentGroup::BlockQuoteGroup { .. } => {
            // This should not happen for ListGroup but handle gracefully
            rsx! { div { "Invalid list group content" } }
        }
    }
}
