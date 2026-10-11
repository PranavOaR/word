use std::path::PathBuf;

use eframe::egui::{
    self, Align, Color32, FontId, Key, Layout, RichText, Sense, Stroke, TextEdit, Vec2,
};

use crate::document::Document;

const APP_BG: Color32 = Color32::from_rgb(242, 244, 248);
const RIBBON_BG: Color32 = Color32::from_rgb(250, 251, 253);
const BLUE: Color32 = Color32::from_rgb(36, 99, 190);
const MUTED: Color32 = Color32::from_rgb(102, 112, 128);

pub struct WordMockApp {
    document: Document,
    zoom: f32,
    active_tab: RibbonTab,
    message: Option<String>,
    pending_action: Option<PendingAction>,
}

#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
}

impl Default for WordMockApp {
    fn default() -> Self {
        Self {
            document: Document::default(),
            zoom: 1.0,
            active_tab: RibbonTab::Home,
            message: None,
            pending_action: None,
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum RibbonTab {
    #[default]
    Home,
    Insert,
    Layout,
    Review,
    View,
}

impl eframe::App for WordMockApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root_ui.ctx().clone();
        self.handle_shortcuts(&ctx);
        let title = if self.document.dirty {
            format!("{} • WordMock", self.document.title())
        } else {
            format!("{} — WordMock", self.document.title())
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::Panel::top("title_bar")
            .exact_size(42.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(31, 78, 145)))
            .show(root_ui, |ui| self.title_bar(ui));

        egui::Panel::top("ribbon")
            .exact_size(126.0)
            .frame(egui::Frame::new().fill(RIBBON_BG))
            .show(root_ui, |ui| self.ribbon(ui));

        egui::Panel::bottom("status_bar")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(224))))
            .show(root_ui, |ui| self.status_bar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(APP_BG))
            .show(root_ui, |ui| self.editor_area(ui));

        self.unsaved_changes_dialog(&ctx);

        if let Some(message) = self.message.clone() {
            egui::Window::new("WordMock")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(&ctx, |ui| {
                    ui.label(message);
                    if ui.button("OK").clicked() {
                        self.message = None;
                    }
                });
        }
    }
}

impl WordMockApp {
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(10.0);
            if ui.small_button(RichText::new("＋ New").color(Color32::WHITE)).clicked() {
                self.new_document();
            }
            if ui.small_button(RichText::new("Open").color(Color32::WHITE)).clicked() {
                self.open_document();
            }
            if ui.small_button(RichText::new("Save").color(Color32::WHITE)).clicked() {
                self.save_document();
            }
            ui.add_space(20.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(14.0);
                let label = if self.document.dirty {
                    format!("{} •", self.document.title())
                } else {
                    self.document.title().to_owned()
                };
                ui.label(RichText::new(label).color(Color32::WHITE).strong());
                ui.add_space(6.0);
                ui.label(RichText::new("W").size(18.0).strong().color(Color32::WHITE));
            });
        });
    }

    fn ribbon(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            for (tab, name) in [
                (RibbonTab::Home, "Home"),
                (RibbonTab::Insert, "Insert"),
                (RibbonTab::Layout, "Layout"),
                (RibbonTab::Review, "Review"),
                (RibbonTab::View, "View"),
            ] {
                let active = self.active_tab == tab;
                let text = RichText::new(name).color(if active { BLUE } else { MUTED });
                let response = ui.add(egui::Button::new(text).frame(false).sense(Sense::click()));
                if response.clicked() {
                    self.active_tab = tab;
                }
                if active {
                    let rect = response.rect;
                    ui.painter().line_segment(
                        [egui::pos2(rect.left(), rect.bottom()), egui::pos2(rect.right(), rect.bottom())],
                        Stroke::new(2.0, BLUE),
                    );
                }
            }
        });
        ui.separator();

        ui.add_space(4.0);
        ui.horizontal(|ui| match self.active_tab {
            RibbonTab::Home => self.home_ribbon(ui),
            RibbonTab::Insert => {
                ui.add_space(12.0);
                ui.label(RichText::new("Insert tools are coming next").color(MUTED));
            }
            RibbonTab::Layout => {
                ui.add_space(12.0);
                ui.label(RichText::new("Page layout controls are coming next").color(MUTED));
            }
            RibbonTab::Review => {
                ui.add_space(12.0);
                ui.label(RichText::new("Review tools are coming next").color(MUTED));
            }
            RibbonTab::View => self.view_ribbon(ui),
        });
    }

    fn home_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Normal").color(Color32::from_gray(45)));
                ui.add_space(18.0);
                ui.label(RichText::new("Default font").color(Color32::from_gray(45)));
                ui.add_space(8.0);
                ui.add(egui::Slider::new(&mut self.document.font_size, 9.0..=28.0).suffix(" pt"));
            });
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                ui.add_enabled(false, egui::Button::new(RichText::new("B").strong().size(16.0)));
                ui.add_enabled(false, egui::Button::new(RichText::new("I").italics().size(16.0)));
                ui.add_enabled(false, egui::Button::new(RichText::new("U").underline().size(16.0)));
                ui.separator();
                if ui.button("•  Bullets").clicked() {
                    self.insert_prefix("• ");
                }
                if ui.button("1.  Numbering").clicked() {
                    self.insert_prefix("1. ");
                }
                ui.separator();
                ui.label(RichText::new("Paragraph").color(MUTED));
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Font size applies to the editor. Rich text formatting is on the roadmap.").size(11.0).color(MUTED));
        });
    }

    fn view_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.label("Zoom");
        ui.add(egui::Slider::new(&mut self.zoom, 0.75..=1.35).show_value(false));
        ui.label(format!("{}%", (self.zoom * 100.0).round() as u32));
        if ui.button("100%").clicked() {
            self.zoom = 1.0;
        }
    }

    fn editor_area(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let page_width = (816.0 * self.zoom).min(available.x - 64.0).max(360.0);
        let page_height = available.y - 32.0;
        ui.allocate_ui_with_layout(available, Layout::top_down(Align::Center), |ui| {
            ui.add_space(16.0);
            let (page_rect, _) = ui.allocate_exact_size(Vec2::new(page_width, page_height), Sense::hover());
            ui.painter().rect_filled(page_rect.translate(Vec2::new(0.0, 2.0)), 2.0, Color32::from_black_alpha(18));
            ui.painter().rect_filled(page_rect, 1.0, Color32::WHITE);
            ui.painter().rect_stroke(page_rect, 1.0, Stroke::new(1.0, Color32::from_gray(222)), egui::StrokeKind::Inside);

            let margin_x = 76.0 * self.zoom;
            let margin_y = 64.0 * self.zoom;
            let editor_rect = egui::Rect::from_min_max(
                page_rect.min + Vec2::new(margin_x, margin_y),
                page_rect.max - Vec2::new(margin_x, margin_y),
            );
            let mut editor_ui = ui.new_child(egui::UiBuilder::new().max_rect(editor_rect).layout(Layout::top_down(Align::Min)));
            let edit = TextEdit::multiline(&mut self.document.text)
                .font(FontId::proportional(self.document.font_size * self.zoom))
                .desired_width(editor_rect.width())
                .desired_rows(1)
                .hint_text("Start writing…")
                .frame(egui::Frame::NONE)
                .margin(Vec2::ZERO);
            let response = editor_ui.add_sized(editor_rect.size(), edit);
            if response.changed() {
                self.document.dirty = true;
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(format!("Page {} of {}", self.document.page_count(), self.document.page_count())).size(11.0).color(MUTED));
            ui.separator();
            ui.label(RichText::new(format!("{} words", self.document.word_count())).size(11.0).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(RichText::new(format!("{}%", (self.zoom * 100.0).round() as u32)).size(11.0).color(MUTED));
                ui.add(egui::Slider::new(&mut self.zoom, 0.75..=1.35).show_value(false).step_by(0.05));
                ui.label(RichText::new("−").color(MUTED));
                ui.add_space(8.0);
                ui.label(RichText::new("English (United States)").size(11.0).color(MUTED));
                ui.add_space(12.0);
            });
        });
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);
        let command = modifiers.command;
        let (new, open, save) = ctx.input(|i| {
            (
                i.key_pressed(Key::N) && command,
                i.key_pressed(Key::O) && command,
                i.key_pressed(Key::S) && command,
            )
        });
        if new {
            self.new_document();
        } else if open {
            self.open_document();
        } else if save {
            self.save_document();
        }
    }

    fn new_document(&mut self) {
        self.request_action(PendingAction::New);
    }

    fn open_document(&mut self) {
        self.request_action(PendingAction::Open);
    }

    fn pick_open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text documents", &["txt", "md"])
            .pick_file()
        {
            match Document::open(path) {
                Ok(document) => self.document = document,
                Err(error) => self.message = Some(format!("Could not open file: {error}")),
            }
        }
    }

    fn save_document(&mut self) -> bool {
        if self.document.path.is_some() {
            return match self.document.save() {
                Ok(()) => {
                    self.message = Some("Document saved.".to_owned());
                    true
                }
                Err(error) => {
                    self.message = Some(format!("Could not save file: {error}"));
                    false
                }
            };
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text document", &["txt"])
            .set_file_name("Untitled.txt")
            .save_file()
        {
            return self.write_to(path);
        }
        false
    }

    fn write_to(&mut self, path: PathBuf) -> bool {
        match self.document.save_as(path) {
            Ok(()) => {
                self.message = Some("Document saved.".to_owned());
                true
            }
            Err(error) => {
                self.message = Some(format!("Could not save file: {error}"));
                false
            }
        }
    }

    fn request_action(&mut self, action: PendingAction) {
        if self.document.dirty {
            self.pending_action = Some(action);
        } else {
            self.perform_action(action);
        }
    }

    fn perform_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::New => self.document.reset(),
            PendingAction::Open => self.pick_open_file(),
        }
    }

    fn unsaved_changes_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_action else {
            return;
        };

        let mut decision = None;
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Would you like to save your changes before continuing?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        decision = Some(0);
                    }
                    if ui.button("Discard").clicked() {
                        decision = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        decision = Some(2);
                    }
                });
            });

        match decision {
            Some(0) => {
                if self.save_document() {
                    self.pending_action = None;
                    self.perform_action(action);
                }
            }
            Some(1) => {
                self.pending_action = None;
                self.perform_action(action);
            }
            Some(2) => self.pending_action = None,
            _ => {}
        }
    }

    fn insert_prefix(&mut self, prefix: &str) {
        if !self.document.text.is_empty() && !self.document.text.ends_with('\n') {
            self.document.text.push('\n');
        }
        self.document.text.push_str(prefix);
        self.document.dirty = true;
    }
}
