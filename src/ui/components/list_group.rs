use crate::editing::ContentGroup as EditorContentGroup;
use crate::ui::components::list_component::ListComponent;
use dioxus::prelude::*;
use std::path::PathBuf;

/// Component to render a list group
#[component]
pub fn ListGroup(
    group: EditorContentGroup,
    group_index: usize,
    document: crate::editing::Document,
    focused_anchor_id: Signal<Option<crate::editing::AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<crate::editing::Cmd>,
) -> Element {
    match group {
        EditorContentGroup::BulletListGroup { items } => {
            rsx! {
                ListComponent {
                    key: "{group_index}-bullet-list",
                    items,
                    list_type: "ul",
                    on_file_select,
                    on_focus: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |block: crate::editing::RenderBlock| {
                            focused_anchor_id.set(Some(block.id));
                        }
                    },
                    on_command,
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
                    on_file_select,
                    on_focus: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |block: crate::editing::RenderBlock| {
                            focused_anchor_id.set(Some(block.id));
                        }
                    },
                    on_command,
                    focused_anchor_id,
                    document
                }
            }
        }
        EditorContentGroup::SingleBlock(_) => {
            // This should not happen for render_list_group but handle gracefully
            rsx! { div { "Invalid list group content" } }
        }
    }
}
