use dioxus::prelude::*;
use markdown_neuraxis::{OutlineItem, parse_markdown_outline};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        let usage = format!("Usage: {} <notes-folder-path>", args[0]);
        return rsx! {
            div { "{usage}" }
        };
    }

    let notes_path = PathBuf::from(&args[1]);
    if !notes_path.exists() || !notes_path.is_dir() {
        let error = format!("Error: '{}' is not a valid directory", args[1]);
        return rsx! {
            div { "{error}" }
        };
    }
    let markdown_files = use_signal(|| scan_markdown_files(&notes_path));
    let mut selected_file = use_signal(|| None::<PathBuf>);
    let mut markdown_content = use_signal(|| String::new());

    rsx! {
        style { {SOLARIZED_LIGHT_CSS} }
        div {
            class: "app-container",
            div {
                class: "sidebar",
                h2 { "Files" }
                p { "Found {markdown_files.read().len()} markdown files:" }
                div {
                    class: "file-list",
                    for file in markdown_files.read().iter() {
                        FileItem {
                            file: file.clone(),
                            notes_path: notes_path.clone(),
                            is_selected: selected_file.read().as_ref() == Some(file),
                            on_select: move |file_path: PathBuf| {
                                match fs::read_to_string(&file_path) {
                                    Ok(content) => {
                                        *markdown_content.write() = content;
                                        *selected_file.write() = Some(file_path);
                                    }
                                    Err(e) => {
                                        eprintln!("Error reading file {:?}: {}", file_path, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                class: "main-content",
                if let Some(ref file) = *selected_file.read() {
                    MainPanel {
                        file: file.clone(),
                        notes_path: notes_path.clone(),
                        content: markdown_content.read().clone()
                    }
                } else {
                    div {
                        class: "welcome",
                        h1 { "markdown-neuraxis" }
                        p { "Select a file from the sidebar to view its content" }
                    }
                }
            }
        }
    }
}

#[component]
fn FileItem(
    file: PathBuf,
    notes_path: PathBuf,
    is_selected: bool,
    on_select: EventHandler<PathBuf>,
) -> Element {
    let pages_path = notes_path.join("pages");
    let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
        relative.to_string_lossy().to_string()
    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        "Unknown".to_string()
    };

    rsx! {
        div {
            class: if is_selected { "file-item selected" } else { "file-item" },
            onclick: move |_| on_select.call(file.clone()),
            "{display_name}"
        }
    }
}

#[component]
fn MainPanel(file: PathBuf, notes_path: PathBuf, content: String) -> Element {
    let pages_path = notes_path.join("pages");
    let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
        relative.to_string_lossy().to_string()
    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        "Selected File".to_string()
    };

    let doc = parse_markdown_outline(&content);

    rsx! {
        h1 { "📝 {display_name}" }
        hr {}
        if !content.is_empty() {
            div {
                class: "outline-container",
                h3 { "Parsed outline:" }
                div {
                    class: "outline-content",
                    for item in &doc.outline {
                        OutlineItemComponent { item: item.clone(), indent: 0 }
                    }
                }
            }
        }
    }
}

#[component]
fn OutlineItemComponent(item: OutlineItem, indent: usize) -> Element {
    let _indent_str = "  ".repeat(indent);

    rsx! {
        div {
            class: "outline-item",
            style: "margin-left: {indent * 20}px;",
            "[{item.level}] {item.content}"
        }
        for child in &item.children {
            OutlineItemComponent { item: child.clone(), indent: indent + 1 }
        }
    }
}

fn scan_markdown_files(notes_path: &Path) -> Vec<PathBuf> {
    let pages_path = notes_path.join("pages");
    if !pages_path.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    scan_directory_recursive(&pages_path, &mut files);
    files.sort();
    files
}

fn scan_directory_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_directory_recursive(&path, files);
            } else if let Some(ext) = path.extension() {
                if ext == "md" {
                    files.push(path);
                }
            }
        }
    }
}

const SOLARIZED_LIGHT_CSS: &str = r#"
:root {
    --base03: #002b36;
    --base02: #073642;
    --base01: #586e75;
    --base00: #657b83;
    --base0: #839496;
    --base1: #93a1a1;
    --base2: #eee8d5;
    --base3: #fdf6e3;
    --yellow: #b58900;
    --orange: #cb4b16;
    --red: #dc322f;
    --magenta: #d33682;
    --violet: #6c71c4;
    --blue: #268bd2;
    --cyan: #2aa198;
    --green: #859900;
}

body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background-color: var(--base3);
    color: var(--base01);
}

.app-container {
    display: flex;
    height: 100vh;
}

.sidebar {
    width: 300px;
    background-color: var(--base2);
    border-right: 1px solid var(--base1);
    padding: 16px;
    overflow-y: auto;
}

.sidebar h2 {
    margin-top: 0;
    color: var(--base01);
}

.file-list {
    margin-top: 12px;
}

.file-item {
    padding: 8px 12px;
    margin: 2px 0;
    cursor: pointer;
    border-radius: 4px;
    transition: background-color 0.2s;
}

.file-item:hover {
    background-color: var(--base1);
}

.file-item.selected {
    background-color: var(--blue);
    color: var(--base3);
}

.main-content {
    flex: 1;
    padding: 16px;
    overflow-y: auto;
}

.welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 50%;
    text-align: center;
}

.outline-container {
    margin-top: 16px;
}

.outline-content {
    margin-top: 12px;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}

.outline-item {
    padding: 2px 0;
    line-height: 1.4;
}

hr {
    border: none;
    border-top: 1px solid var(--base1);
    margin: 16px 0;
}
"#;
