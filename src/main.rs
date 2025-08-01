use eframe::egui;
use markdown_neuraxis::{OutlineItem, parse_markdown_outline};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "markdown-neuraxis",
        options,
        Box::new(|_cc| Ok(Box::new(MarkdownApp::default()))),
    )
}

#[derive(Default)]
struct MarkdownApp {
    markdown_input: String,
}

impl eframe::App for MarkdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
