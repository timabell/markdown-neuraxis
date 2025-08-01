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
}

impl MarkdownApp {
    fn new(notes_path: PathBuf) -> Self {
        let markdown_files = scan_markdown_files(&notes_path);
        Self {
            notes_path,
            markdown_files,
            markdown_input: String::new(),
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

            egui::ScrollArea::vertical().show(ui, |ui| {
                for file in &self.markdown_files {
                    if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                        ui.label(name);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("markdown-neuraxis");

            ui.horizontal(|ui| {
                ui.label("Markdown:");
                ui.text_edit_multiline(&mut self.markdown_input);
            });

            ui.separator();

            if !self.markdown_input.is_empty() {
                let doc = parse_markdown_outline(&self.markdown_input);
                ui.label("Parsed outline:");
                for item in &doc.outline {
                    show_outline_item(ui, item, 0);
                }
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
