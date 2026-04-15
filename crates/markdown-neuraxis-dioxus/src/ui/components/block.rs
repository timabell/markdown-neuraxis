use crate::ui::components::{
    block_quote::BlockQuote, code_fence::CodeFence, editor_block::EditorBlock, heading::Heading,
    paragraph::Paragraph, text_segment::InlineSegments, thematic_break::ThematicBreak,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{
    AnchorId, Block, BlockContent, BlockKind, CheckboxState, Cmd,
};
use std::collections::HashSet;

/// Render a checkbox for task list items
fn render_checkbox(checkbox: &Option<CheckboxState>, on_command: Callback<Cmd>) -> Element {
    let Some(cb) = checkbox else {
        return rsx! {};
    };

    let checked = cb.checked;
    let byte_range = cb.byte_range.clone();

    rsx! {
        input {
            r#type: "checkbox",
            class: "checkbox",
            checked: checked,
            onclick: move |evt| {
                evt.stop_propagation();
                let new_text = if checked { "[ ]" } else { "[x]" };
                on_command.call(Cmd::ReplaceRange {
                    range: byte_range.clone(),
                    text: new_text.to_string(),
                });
            }
        }
    }
}

/// Collapse toggle component for blocks with children
#[component]
pub fn CollapseToggle(
    block_id: AnchorId,
    is_collapsed: bool,
    collapsed_ids: Signal<HashSet<AnchorId>>,
    on_context_menu: Option<Callback<(AnchorId, f64, f64)>>,
) -> Element {
    rsx! {
        span {
            class: "collapse-toggle",
            onclick: {
                let mut collapsed_ids = collapsed_ids;
                move |evt| {
                    evt.stop_propagation();
                    let mut ids = collapsed_ids.write();
                    if ids.contains(&block_id) {
                        ids.remove(&block_id);
                    } else {
                        ids.insert(block_id);
                    }
                }
            },
            oncontextmenu: move |evt: Event<MouseData>| {
                evt.prevent_default();
                evt.stop_propagation();
                if let Some(ref cb) = on_context_menu {
                    let coords = evt.client_coordinates();
                    cb.call((block_id, coords.x, coords.y));
                }
            },
            if is_collapsed { "▶" } else { "▼" }
        }
    }
}

#[component]
pub fn BlockRenderer(
    block: Block,
    source: String,
    focused_anchor_id: Signal<Option<AnchorId>>,
    collapsed_ids: Signal<HashSet<AnchorId>>,
    on_context_menu: Option<Callback<(AnchorId, f64, f64)>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);
    let is_collapsed = collapsed_ids.read().contains(&block.id);

    match &block.kind {
        BlockKind::Root => {
            // Container: render children
            if let BlockContent::Children(children) = &block.content {
                rsx! {
                    for (i, child) in children.iter().enumerate() {
                        BlockRenderer {
                            key: "{i}",
                            block: child.clone(),
                            source: source.clone(),
                            focused_anchor_id,
                            collapsed_ids,
                            on_context_menu,
                            on_command,
                            on_wikilink_click
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::List { ordered } => {
            // Container: render list items
            if let BlockContent::Children(children) = &block.content {
                if *ordered {
                    rsx! {
                        ol {
                            class: "list",
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    collapsed_ids,
                                    on_context_menu,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                } else {
                    rsx! {
                        ul {
                            class: "list",
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    collapsed_ids,
                                    on_context_menu,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::ListItem { checkbox, .. } => {
            let has_children = matches!(&block.content, BlockContent::Children(c) if !c.is_empty());
            let block_id = block.id;
            let checkbox = checkbox.clone();

            if is_focused {
                // Use content_range() - excludes nested children
                let edit_range = block.content_range();
                let content_text = source.get(edit_range.clone()).unwrap_or("").to_string();
                let edit_range = Some(edit_range);
                let block_clone = block.clone();
                rsx! {
                    li {
                        class: "list-item",
                        if has_children {
                            CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                        }
                        EditorBlock {
                            block: block_clone,
                            content_text,
                            edit_range,
                            on_command,
                            on_cancel: {
                                let mut focused_anchor_id = focused_anchor_id;
                                move |_| focused_anchor_id.set(None)
                            }
                        }
                        // Still render nested children below the editor (unless collapsed)
                        if !is_collapsed {
                            if let BlockContent::Children(children) = &block.content {
                                for (i, child) in children.iter().enumerate() {
                                    BlockRenderer {
                                        key: "{i}",
                                        block: child.clone(),
                                        source: source.clone(),
                                        focused_anchor_id,
                                        collapsed_ids,
                                        on_context_menu,
                                        on_command,
                                        on_wikilink_click
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let segments = block.segments.clone();
                let list_class = if checkbox.is_some() {
                    "list-item has-checkbox"
                } else {
                    "list-item"
                };
                rsx! {
                    li {
                        class: "{list_class}",
                        if has_children {
                            CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                        }
                        // Render checkbox if present
                        {render_checkbox(&checkbox, on_command)}
                        span {
                            class: "list-item-content clickable-block",
                            onclick: {
                                let mut focused_anchor_id = focused_anchor_id;
                                move |evt| {
                                    evt.stop_propagation();
                                    focused_anchor_id.set(Some(block_id))
                                }
                            },
                            InlineSegments {
                                segments,
                                on_wikilink_click
                            }
                        }
                        // Render nested children (nested lists) unless collapsed
                        if !is_collapsed {
                            if let BlockContent::Children(children) = &block.content {
                                for (i, child) in children.iter().enumerate() {
                                    BlockRenderer {
                                        key: "{i}",
                                        block: child.clone(),
                                        source: source.clone(),
                                        focused_anchor_id,
                                        collapsed_ids,
                                        on_context_menu,
                                        on_command,
                                        on_wikilink_click
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        BlockKind::Heading { level } => rsx! {
            Heading {
                block: block.clone(),
                source: source.clone(),
                level: *level,
                focused_anchor_id,
                collapsed_ids,
                on_context_menu,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::Paragraph => rsx! {
            Paragraph {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::FencedCode { language } => rsx! {
            CodeFence {
                block: block.clone(),
                source: source.clone(),
                lang: language.clone(),
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::BlockQuote => rsx! {
            BlockQuote {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                collapsed_ids,
                on_context_menu,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::ThematicBreak => rsx! {
            ThematicBreak {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                on_command
            }
        },
        BlockKind::Table => {
            let block_id = block.id;
            if is_focused {
                // Edit entire table as raw markdown
                let content_text = source
                    .get(block.node_range.clone())
                    .unwrap_or("")
                    .to_string();
                let block_clone = block.clone();
                rsx! {
                    div {
                        class: "table-container clickable-block",
                        EditorBlock {
                            block: block_clone,
                            content_text,
                            on_command,
                            on_cancel: {
                                let mut focused_anchor_id = focused_anchor_id;
                                move |_| focused_anchor_id.set(None)
                            }
                        }
                    }
                }
            } else if let BlockContent::Children(children) = &block.content {
                rsx! {
                    table {
                        class: "table clickable-block",
                        onclick: {
                            let mut focused_anchor_id = focused_anchor_id;
                            move |evt| {
                                evt.stop_propagation();
                                focused_anchor_id.set(Some(block_id))
                            }
                        },
                        for (i, child) in children.iter().enumerate() {
                            BlockRenderer {
                                key: "{i}",
                                block: child.clone(),
                                source: source.clone(),
                                focused_anchor_id,
                                collapsed_ids,
                                on_context_menu,
                                on_command,
                                on_wikilink_click
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::TableRow { is_header } => {
            if let BlockContent::Children(children) = &block.content {
                rsx! {
                    tr {
                        class: if *is_header { "table-header-row" } else { "table-row" },
                        for (i, child) in children.iter().enumerate() {
                            BlockRenderer {
                                key: "{i}",
                                block: child.clone(),
                                source: source.clone(),
                                focused_anchor_id,
                                collapsed_ids,
                                on_context_menu,
                                on_command,
                                on_wikilink_click
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::TableCell => {
            let segments = block.segments.clone();
            rsx! {
                td {
                    class: "table-cell",
                    InlineSegments {
                        segments,
                        on_wikilink_click
                    }
                }
            }
        }
    }
}
