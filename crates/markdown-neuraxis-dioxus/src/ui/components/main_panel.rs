use crate::ui::components::EmptyDocument;
use crate::ui::components::document_content::DocumentContent;
use dioxus::events::Key;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, Snapshot};
use markdown_neuraxis_engine::models::MarkdownFile;
use std::path::PathBuf;

#[component]
pub fn MainPanel(
    file: MarkdownFile,
    snapshot: Snapshot,
    document: Document,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<()>,
    on_command: Callback<Cmd>,
) -> Element {
    let mut focused_anchor_id = use_signal(|| None::<AnchorId>);
    let snapshot_clone = snapshot.clone();
    let mut navigate_to_block = create_navigation_handler(focused_anchor_id, snapshot_clone);
    let display_name = file.display_path();

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
            h1 { "📝 {display_name}" }
            hr {}
            if !snapshot.blocks.is_empty() {
                DocumentContent {
                    snapshot: snapshot_for_content,
                    document,
                    focused_anchor_id,
                    on_file_select,
                    on_command
                }
            } else {
                EmptyDocument { on_command }
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
