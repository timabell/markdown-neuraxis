use crate::ui::components::{
    block_quote::BlockQuote, code_fence::CodeFence, heading::Heading, html_block::HtmlBlock,
    paragraph::Paragraph, thematic_break::ThematicBreak, unhandled_markdown::UnhandledMarkdown,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{BlockKind, RenderBlock};
use std::path::PathBuf;

#[component]
pub fn Block(
    block: RenderBlock,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<()>,
    on_wikilink_click: Callback<String>,
) -> Element {
    match &block.kind {
        BlockKind::Heading { level } => rsx! {
            Heading {
                block: block.clone(),
                level: (*level).into(),
                notes_path,
                on_focus,
                on_wikilink_click
            }
        },
        BlockKind::Paragraph => rsx! {
            Paragraph {
                block: block.clone(),
                notes_path,
                on_focus,
                on_wikilink_click
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
        BlockKind::HtmlBlock => rsx! {
            HtmlBlock {
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
