use eframe::egui;
use markdown_neuraxis::{OutlineItem, parse_markdown_outline};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <notes-folder-path>", args[0]);
        std::process::exit(1);
    }

    let notes_path = PathBuf::from(&args[1]);
    if !notes_path.exists() || !notes_path.is_dir() {
        eprintln!("Error: '{}' is not a valid directory", args[1]);
        std::process::exit(1);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "markdown-neuraxis",
        options,
        Box::new(move |_cc| Ok(Box::new(MarkdownApp::new(notes_path)))),
    )
}

struct MarkdownApp {
    notes_path: PathBuf,
    markdown_files: Vec<PathBuf>,
    markdown_input: String,
    selected_file: Option<PathBuf>,
}

impl MarkdownApp {
    fn new(notes_path: PathBuf) -> Self {
        let markdown_files = scan_markdown_files(&notes_path);
        Self {
            notes_path,
            markdown_files,
            markdown_input: String::new(),
            selected_file: None,
        }
    }

    fn load_file(&mut self, file_path: &Path) {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                self.markdown_input = content;
                self.selected_file = Some(file_path.to_path_buf());
            }
            Err(e) => {
                eprintln!("Error reading file {:?}: {}", file_path, e);
            }
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

impl eframe::App for MarkdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("file_browser").show(ctx, |ui| {
            ui.heading("Files");
            ui.label(format!(
                "Found {} markdown files:",
                self.markdown_files.len()
            ));

            let mut file_to_load = None;
            let pages_path = self.notes_path.join("pages");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for file in &self.markdown_files {
                    let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
                        relative.to_string_lossy().to_string()
                    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                        name.to_string()
                    } else {
                        "Unknown".to_string()
                    };

                    let is_selected = self.selected_file.as_ref() == Some(file);
                    let response = ui.selectable_label(is_selected, display_name);
                    if response.clicked() {
                        file_to_load = Some(file.clone());
                    }
                }
            });

            if let Some(file) = file_to_load {
                self.load_file(&file);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref file) = self.selected_file {
                let pages_path = self.notes_path.join("pages");
                let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
                    relative.to_string_lossy().to_string()
                } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                    name.to_string()
                } else {
                    "Selected File".to_string()
                };
                ui.heading(format!("📝 {}", display_name));
            } else {
                ui.heading("markdown-neuraxis");
                ui.label("Select a file from the sidebar to view its content");
            }

            ui.separator();

            if !self.markdown_input.is_empty() {
                let doc = parse_markdown_outline(&self.markdown_input);
                ui.label("Parsed outline:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for item in &doc.outline {
                        show_outline_item(ui, item, 0);
                    }
                });
            }
        });
    }
}

fn show_outline_item(ui: &mut egui::Ui, item: &OutlineItem, indent: usize) {
    let indent_str = "  ".repeat(indent);
    ui.label(format!("{}[{}] {}", indent_str, item.level, item.content));
    for child in &item.children {
        show_outline_item(ui, child, indent + 1);
    }
}
