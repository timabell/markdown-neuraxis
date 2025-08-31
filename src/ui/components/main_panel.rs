use crate::models::{
    BlockId, BulletOperation, ContentBlock, Document, DocumentState, MarkdownFile,
};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn EditableMainPanel(
    file: MarkdownFile,
    document_state: DocumentState,
    on_file_select: Option<Callback<PathBuf>>,
    on_save: Callback<DocumentState>,
) -> Element {
    let display_name = file.display_path();

    // Keep the old editing system for non-bullet blocks (paragraphs, headings, etc)
    let edit_state = document_state.clone();
    let handle_edit = Callback::new(move |block_id: BlockId| {
        let mut new_state = edit_state.clone();
        new_state.start_editing(block_id);
        on_save.call(new_state);
    });

    let save_state = document_state.clone();
    let handle_save = Callback::new(move |(block_id, content): (BlockId, String)| {
        let mut new_state = save_state.clone();
        let _new_block_ids = new_state.finish_editing(block_id, content);
        on_save.call(new_state);
    });

    // Handler for bullet operations
    let operation_state = document_state.clone();
    let handle_bullet_operation = Callback::new(move |operation: BulletOperation| {
        let mut new_state = operation_state.clone();

        // Execute the operation
        if new_state.execute_bullet_operation(operation) {
            on_save.call(new_state);
        }
    });

    let add_block_state = document_state.clone();
    let handle_add_block = Callback::new(move |_| {
        let mut new_state = add_block_state.clone();
        // Add an empty paragraph at the end and start editing it
        let new_block = crate::models::ContentBlock::Paragraph {
            segments: vec![crate::models::TextSegment::Text("".to_string())],
        };
        let new_block_id = new_state.insert_block_at_end(new_block);
        new_state.start_editing(new_block_id);
        on_save.call(new_state);
    });

    // Handle document-level keyboard navigation
    let nav_state = document_state.clone();
    let handle_keydown = Callback::new(move |evt: KeyboardEvent| {
        // Only handle navigation keys if we're not currently editing
        if nav_state.editing_block.is_none()
            && !evt.data().modifiers().ctrl()
            && !evt.data().modifiers().shift()
            && !evt.data().modifiers().alt()
        {
            match evt.key() {
                Key::ArrowUp => {
                    let mut new_state = nav_state.clone();
                    new_state.select_previous_block();
                    on_save.call(new_state);
                    evt.prevent_default();
                }
                Key::ArrowDown => {
                    let mut new_state = nav_state.clone();
                    new_state.select_next_block();
                    on_save.call(new_state);
                    evt.prevent_default();
                }
                Key::Enter => {
                    let mut new_state = nav_state.clone();
                    if new_state.start_editing_selected() {
                        on_save.call(new_state);
                    }
                    evt.prevent_default();
                }
                _ => {}
            }
        }
    });

    // Create a reference to the document container for focus management
    let mut document_ref = use_signal(|| None::<std::rc::Rc<MountedData>>);

    // Handle focus request when editing ends
    let handle_focus_document = Callback::new(move |_| {
        if let Some(mounted) = document_ref.read().clone() {
            spawn(async move {
                let _ = mounted.set_focus(true).await;
            });
        }
    });

    rsx! {
        div {
            class: "document-container",
            tabindex: "0", // Make div focusable for keyboard events
            onkeydown: handle_keydown,
            autofocus: true,
            onmounted: move |evt| {
                document_ref.set(Some(evt.data()));
            },

            h1 { "📝 {display_name}" }
            hr {}
            if !document_state.blocks.is_empty() {
                div {
                    class: "document-content",
                    for (block_id, block) in &document_state.blocks {
                        {
                            let is_editing = document_state.is_editing(*block_id).is_some();
                            let is_selected = document_state.selected_block() == Some(*block_id);
                            rsx! {
                                super::EditableBlock {
                                    // Addition of the key forces component recreation when BlockId changes.
                                    // When blocks are split (1 -> N blocks), each gets a new UUID-based BlockId.
                                    // Without this key, Dioxus reuses components and editing signals retain stale content.
                                    // With this key, split blocks get fresh components with correct initial content.
                                    // NOTE: This is similar to React's key prop but Dioxus uses it for component identity,
                                    // ensuring use_signal() gets re-initialized with the correct block content.
                                    // Also include editing state in key to force recreation when editing state changes.
                                    key: "{block_id:?}-{is_editing}-{is_selected}",
                                    block: block.clone(),
                                    block_id: *block_id,
                                    editing_raw: document_state.is_editing(*block_id).cloned(),
                                    is_selected: is_selected,
                                    on_edit: handle_edit,
                                    on_save: handle_save,
                                    on_bullet_operation: Some(handle_bullet_operation),
                                    on_editing_end: Some(handle_focus_document),
                                    on_file_select: on_file_select,
                                    document_state: document_state.clone()
                                }
                            }
                        }
                    }
                    // Add block button at the end of the document
                    div {
                        class: "add-block-container",
                        button {
                            class: "add-block-button",
                            onclick: handle_add_block,
                            "+"
                        }
                    }
                }
            } else {
                div {
                    class: "empty-document",
                    p { "This document appears to be empty." }
                    button {
                        class: "add-block-button",
                        onclick: handle_add_block,
                        "Add first block +"
                    }
                }
            }
        }
    }
}

#[component]
pub fn MainPanel(
    file: MarkdownFile,
    document: Document,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    let display_name = file.display_path();

    rsx! {
        h1 { "📝 {display_name}" }
        hr {}
        if !document.content.is_empty() {
            div {
                class: "document-content",
                for block in &document.content {
                    ContentBlockComponent { block: block.clone(), on_file_select: on_file_select }
                }
            }
        } else {
            div {
                class: "empty-document",
                p { "This document appears to be empty." }
            }
        }
    }
}

#[component]
pub fn ContentBlockComponent(
    block: ContentBlock,
    on_file_select: Option<Callback<PathBuf>>,
) -> Element {
    match block {
        ContentBlock::Heading { level, text } => {
            let class_name = format!("heading level-{level}");
            match level {
                1 => rsx! { h1 { class: "{class_name}", "{text}" } },
                2 => rsx! { h2 { class: "{class_name}", "{text}" } },
                3 => rsx! { h3 { class: "{class_name}", "{text}" } },
                4 => rsx! { h4 { class: "{class_name}", "{text}" } },
                5 => rsx! { h5 { class: "{class_name}", "{text}" } },
                _ => rsx! { h6 { class: "{class_name}", "{text}" } },
            }
        }
        ContentBlock::Paragraph { segments } => {
            rsx! {
                p {
                    class: "paragraph",
                    for segment in segments {
                        super::TextSegmentComponent { segment: segment.clone(), on_file_select: on_file_select }
                    }
                }
            }
        }
        ContentBlock::BulletList { items } => {
            rsx! {
                div {
                    class: "bullet-list",
                    for (_block_id, item) in items {
                        super::BulletItemComponent {
                            item: item.clone(),
                            indent: 0,
                            is_numbered: false,
                            item_number: None,
                            on_file_select: on_file_select
                        }
                    }
                }
            }
        }
        ContentBlock::NumberedList { items } => {
            rsx! {
                div {
                    class: "numbered-list",
                    for (idx, (_block_id, item)) in items.iter().enumerate() {
                        super::BulletItemComponent {
                            item: item.clone(),
                            indent: 0,
                            is_numbered: true,
                            item_number: Some(idx + 1),
                            on_file_select: on_file_select
                        }
                    }
                }
            }
        }
        ContentBlock::CodeBlock { language, code } => {
            let code_class = if let Some(ref lang) = language {
                format!("language-{lang}")
            } else {
                "language-text".to_string()
            };

            rsx! {
                div {
                    class: "code-block",
                    if let Some(lang) = language {
                        div { class: "code-language", "{lang}" }
                    }
                    pre {
                        code {
                            class: "{code_class}",
                            "{code}"
                        }
                    }
                }
            }
        }
        ContentBlock::Quote(text) => {
            rsx! {
                blockquote { class: "quote", "{text}" }
            }
        }
        ContentBlock::Rule => {
            rsx! {
                hr { class: "rule" }
            }
        }
    }
}
