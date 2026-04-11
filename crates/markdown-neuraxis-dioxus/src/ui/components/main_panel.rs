use crate::ui::components::EmptyDocument;
use crate::ui::components::document_content::DocumentContent;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, Snapshot};
use markdown_neuraxis_engine::models::MarkdownFile;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[component]
pub fn MainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    notes_path: PathBuf,
    document: Arc<Document>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
    on_rename: Callback<String>,
    #[props(default = false)] is_new_file: bool,
) -> Element {
    let mut focused_anchor_id = use_signal(|| None::<AnchorId>);
    let collapsed_ids = use_signal(HashSet::<AnchorId>::new);
    let context_menu_position = use_signal(|| None::<(f64, f64)>);
    let context_menu_block = use_signal(|| None::<AnchorId>);
    let snapshot_clone = snapshot.clone();
    let mut navigate_to_block = create_navigation_handler(focused_anchor_id, snapshot_clone);

    // Editable filename state (display path without .md extension)
    let initial_path = file.display_path().to_string();
    let initial_path_for_keydown = initial_path.clone();
    let initial_path_for_blur = initial_path.clone();
    let initial_path_for_click = initial_path.clone();
    // For new files, start in title editing mode; otherwise focus content
    let mut editing_name = use_signal(|| is_new_file);
    let mut name_input = use_signal(|| initial_path.clone());
    let mut focus_content = use_signal(|| !is_new_file);

    // Clone values before using in RSX
    let snapshot_for_keydown = snapshot.clone();
    let snapshot_for_content = snapshot.clone();

    rsx! {
        div {
            class: "document-container",
            tabindex: "0",
            onkeydown: {
                move |event| {
                    handle_document_keydown(event, &mut focused_anchor_id, &snapshot_for_keydown, &mut navigate_to_block);
                }
            },
            if *editing_name.read() {
                input {
                    class: "document-title-input",
                    r#type: "text",
                    value: name_input.read().clone(),
                    onmounted: move |event: Event<MountedData>| async move {
                        let _ = event.data().set_focus(true).await;
                    },
                    oninput: move |event: Event<FormData>| {
                        name_input.set(event.value());
                    },
                    onkeydown: move |event: Event<KeyboardData>| {
                        match event.key() {
                            Key::Enter => {
                                event.prevent_default();
                                let new_path = name_input.read().clone();
                                if new_path != initial_path_for_keydown && !new_path.trim().is_empty() {
                                    on_rename.call(new_path);
                                }
                                editing_name.set(false);
                                // Only focus content area for new unsaved files
                                if is_new_file {
                                    focus_content.set(true);
                                }
                            }
                            Key::Escape => {
                                // Cancel editing, revert to original path
                                event.prevent_default();
                                name_input.set(initial_path_for_keydown.clone());
                                editing_name.set(false);
                            }
                            _ => {}
                        }
                    },
                    onblur: move |_| {
                        let new_path = name_input.read().clone();
                        if new_path != initial_path_for_blur && !new_path.trim().is_empty() {
                            on_rename.call(new_path);
                        }
                        editing_name.set(false);
                    },
                }
            } else {
                h1 {
                    class: "document-title",
                    onclick: move |_| {
                        name_input.set(initial_path_for_click.clone());
                        editing_name.set(true);
                    },
                    "{initial_path}"
                }
            }
            hr {}
            // Show EmptyDocument if no blocks or content is just whitespace
            if !snapshot.blocks.is_empty() && !document.text().trim().is_empty() {
                DocumentContent {
                    snapshot: snapshot_for_content,
                    source: document.text(),
                    notes_path,
                    document,
                    focused_anchor_id,
                    collapsed_ids,
                    context_menu_position,
                    context_menu_block,
                    on_file_select,
                    on_command,
                    on_wikilink_click
                }
            } else {
                // Key changes when focus_content changes, forcing remount to trigger onmounted focus
                EmptyDocument {
                    key: "{focus_content.read()}",
                    on_command,
                    should_focus: *focus_content.read()
                }
            }
        }
    }
}

fn create_navigation_handler(
    mut focused_anchor_id: Signal<Option<AnchorId>>,
    snapshot: Snapshot,
) -> impl FnMut(i32) {
    move |direction: i32| {
        navigate_block(&snapshot, &mut focused_anchor_id, direction);
    }
}

fn navigate_block(
    snapshot: &Snapshot,
    focused_anchor_id: &mut Signal<Option<AnchorId>>,
    direction: i32,
) {
    if snapshot.blocks.is_empty() {
        return;
    }

    let current_focus = *focused_anchor_id.read();

    if let Some(current_id) = current_focus {
        navigate_from_current_focus(snapshot, focused_anchor_id, current_id, direction);
        return;
    }

    focus_first_or_last_block(snapshot, focused_anchor_id, direction);
}

fn navigate_from_current_focus(
    snapshot: &Snapshot,
    focused_anchor_id: &mut Signal<Option<AnchorId>>,
    current_id: AnchorId,
    direction: i32,
) {
    let Some(current_index) = snapshot.blocks.iter().position(|b| b.id == current_id) else {
        return;
    };

    let next_index = (current_index as i32 + direction).max(0) as usize;
    if next_index < snapshot.blocks.len() {
        focused_anchor_id.set(Some(snapshot.blocks[next_index].id));
    }
}

fn focus_first_or_last_block(
    snapshot: &Snapshot,
    focused_anchor_id: &mut Signal<Option<AnchorId>>,
    direction: i32,
) {
    let index = if direction > 0 {
        0
    } else {
        snapshot.blocks.len() - 1
    };
    focused_anchor_id.set(Some(snapshot.blocks[index].id));
}

fn handle_document_keydown(
    event: Event<KeyboardData>,
    focused_anchor_id: &mut Signal<Option<AnchorId>>,
    snapshot: &Snapshot,
    navigate_to_block: &mut impl FnMut(i32),
) {
    if focused_anchor_id.read().is_some() {
        return;
    }

    match event.key() {
        Key::Tab => handle_tab_navigation(event, navigate_to_block),
        Key::Enter => handle_enter_key(focused_anchor_id, snapshot),
        Key::ArrowDown => handle_arrow_navigation(event, navigate_to_block, 1),
        Key::ArrowUp => handle_arrow_navigation(event, navigate_to_block, -1),
        _ => {}
    }
}

fn handle_tab_navigation(event: Event<KeyboardData>, navigate_to_block: &mut impl FnMut(i32)) {
    event.prevent_default();
    let direction = if event.modifiers().shift() { -1 } else { 1 };
    navigate_to_block(direction);
}

fn handle_enter_key(focused_anchor_id: &mut Signal<Option<AnchorId>>, snapshot: &Snapshot) {
    if !snapshot.blocks.is_empty() {
        focused_anchor_id.set(Some(snapshot.blocks[0].id));
    }
}

fn handle_arrow_navigation(
    event: Event<KeyboardData>,
    navigate_to_block: &mut impl FnMut(i32),
    direction: i32,
) {
    event.prevent_default();
    navigate_to_block(direction);
}
