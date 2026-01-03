use dioxus::prelude::*;

/// Full-screen error display component for showing errors visibly on Android and desktop
#[component]
pub fn ErrorScreen(title: String, message: String, details: Option<String>) -> Element {
    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; padding: 24px; text-align: center;",
            h1 {
                style: "color: #dc322f;",
                "{title}"
            }
            p { "{message}" }
            if let Some(ref detail_text) = details {
                pre {
                    style: "text-align: left; white-space: pre-wrap; word-break: break-word; margin-top: 16px;",
                    "{detail_text}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::dioxus_core::VirtualDom;
    use dioxus_ssr::render;

    #[test]
    fn test_error_screen_renders_title_and_message() {
        let mut dom = VirtualDom::new_with_props(
            ErrorScreen,
            ErrorScreenProps {
                title: "Test Error".to_string(),
                message: "Something went wrong".to_string(),
                details: None,
            },
        );
        dom.rebuild_in_place();
        let html = render(&dom);

        assert!(html.contains("Test Error"));
        assert!(html.contains("Something went wrong"));
    }

    #[test]
    fn test_error_screen_renders_with_details() {
        let mut dom = VirtualDom::new_with_props(
            ErrorScreen,
            ErrorScreenProps {
                title: "Config Error".to_string(),
                message: "Failed to load configuration".to_string(),
                details: Some("File not found: /path/to/config".to_string()),
            },
        );
        dom.rebuild_in_place();
        let html = render(&dom);

        assert!(html.contains("Config Error"));
        assert!(html.contains("Failed to load configuration"));
        assert!(html.contains("File not found: /path/to/config"));
    }
}
