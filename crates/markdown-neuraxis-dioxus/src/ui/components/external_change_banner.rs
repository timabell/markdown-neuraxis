use dioxus::prelude::*;

/// Banner shown above the editor when the open file has changed on
/// disk while the user is editing (ADR-0014 §6). Offers two recovery
/// actions: drop the in-progress edit and reload the disk version, or
/// write the current buffer to a sibling `.mdnx-conflict-*.md` file
/// before reloading.
#[component]
pub fn ExternalChangeBanner(
    file_display_path: String,
    on_discard: Callback<()>,
    on_save_copy: Callback<()>,
) -> Element {
    rsx! {
        div {
            style: concat!(
                "background: #b58900; color: #fdf6e3; padding: 8px 16px; ",
                "display: flex; justify-content: space-between; align-items: center; ",
                "gap: 12px; flex-wrap: wrap;",
            ),
            span {
                "'{file_display_path}' changed on disk while you were editing."
            }
            div {
                style: "display: flex; gap: 8px;",
                button {
                    onclick: move |_| on_discard.call(()),
                    "Discard mine"
                }
                button {
                    onclick: move |_| on_save_copy.call(()),
                    "Save copy"
                }
            }
        }
    }
}
