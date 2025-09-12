use crate::editing::{BlockKind, RenderBlock};
use crate::ui::components::{
    block_quote::BlockQuote, code_fence::CodeFence, heading::Heading, paragraph::Paragraph,
    thematic_break::ThematicBreak, unhandled_markdown::UnhandledMarkdown,
};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn Block(
    block: RenderBlock,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<()>,
) -> Element {
    match &block.kind {
        BlockKind::Heading { level } => rsx! {
            Heading {
                block: block.clone(),
                level: (*level).into(),
                on_focus
            }
        },
        BlockKind::Paragraph => rsx! {
            Paragraph {
                block: block.clone(),
                on_focus
            }
        },
        BlockKind::ListItem { .. } => {
            panic!(
                "ListItem blocks should be grouped into proper ul/ol structure, not rendered individually"
            )
        }
        BlockKind::CodeFence { lang } => rsx! {
            CodeFence {
                block: block.clone(),
                lang: lang.clone(),
                on_focus
            }
        },
        BlockKind::ThematicBreak => rsx! {
            ThematicBreak {
                block: block.clone(),
                on_focus
            }
        },
        BlockKind::BlockQuote => rsx! {
            BlockQuote {
                block: block.clone(),
                on_focus
            }
        },
        BlockKind::UnhandledMarkdown => rsx! {
            UnhandledMarkdown {
                block: block.clone(),
                on_focus
            }
        },
    }
}
