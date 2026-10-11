use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::TextBuffer;
use eframe::egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use eframe::egui::text::{FontData, LayoutJob, TextFormat};
use eframe::egui::{
    self, Align, Color32, FontFamily, FontId, Key, Layout, RichText, Sense, Stroke, TextEdit, Vec2,
};

use crate::document::{CharacterStyle, Document, StyleKind};

const APP_BG: Color32 = Color32::from_rgb(242, 244, 248);
const RIBBON_BG: Color32 = Color32::from_rgb(250, 251, 253);
const BLUE: Color32 = Color32::from_rgb(36, 99, 190);
const MUTED: Color32 = Color32::from_rgb(102, 112, 128);
const EDITOR_ID: &str = "wordmock-document-editor";
const BOLD_FONT_FAMILY: &str = "wordmock-bold";

pub struct WordMockApp {
    document: Document,
    zoom: f32,
    active_tab: RibbonTab,
    message: Option<String>,
    pending_action: Option<PendingAction>,
    selection_chars: Option<Range<usize>>,
    cursor_char: usize,
    typing_style: CharacterStyle,
    bold_font_available: bool,
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
            selection_chars: None,
            cursor_char: 0,
            typing_style: CharacterStyle::default(),
            bold_font_available: false,
        }
    }
}

impl WordMockApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut app = Self::default();
        app.bold_font_available = install_bold_font(ctx);
        app
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
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_gray(224))),
            )
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
            if ui
                .small_button(RichText::new("＋ New").color(Color32::WHITE))
                .clicked()
            {
                self.new_document();
            }
            if ui
                .small_button(RichText::new("Open").color(Color32::WHITE))
                .clicked()
            {
                self.open_document();
            }
            if ui
                .small_button(RichText::new("Save").color(Color32::WHITE))
                .clicked()
            {
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
                        [
                            egui::pos2(rect.left(), rect.bottom()),
                            egui::pos2(rect.right(), rect.bottom()),
                        ],
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
                let mut font_size = self.document.font_size;
                if ui
                    .add(egui::Slider::new(&mut font_size, 9.0..=28.0).suffix(" pt"))
                    .changed()
                {
                    self.document.set_font_size(font_size);
                }
            });
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                let selected = self.selected_byte_range();
                let bold = selected
                    .as_ref()
                    .map_or(self.typing_style.bold, |range| self.document.has_style(range.clone(), StyleKind::Bold));
                let italic = selected
                    .as_ref()
                    .map_or(self.typing_style.italic, |range| self.document.has_style(range.clone(), StyleKind::Italic));
                let underline = selected.as_ref().map_or(self.typing_style.underline, |range| {
                    self.document.has_style(range.clone(), StyleKind::Underline)
                });

                if ui
                    .selectable_label(bold, RichText::new("B").strong().size(16.0))
                    .on_hover_text("Bold (⌘B / Ctrl+B)")
                    .clicked()
                {
                    self.toggle_style(StyleKind::Bold);
                }
                if ui
                    .selectable_label(italic, RichText::new("I").italics().size(16.0))
                    .on_hover_text("Italic (⌘I / Ctrl+I)")
                    .clicked()
                {
                    self.toggle_style(StyleKind::Italic);
                }
                if ui
                    .selectable_label(underline, RichText::new("U").underline().size(16.0))
                    .on_hover_text("Underline (⌘U / Ctrl+U)")
                    .clicked()
                {
                    self.toggle_style(StyleKind::Underline);
                }
                ui.separator();
                if ui.add_enabled(self.document.can_undo(), egui::Button::new("↶ Undo")).clicked() {
                    self.document.undo();
                    self.sync_typing_style_to_cursor();
                }
                if ui.add_enabled(self.document.can_redo(), egui::Button::new("↷ Redo")).clicked() {
                    self.document.redo();
                    self.sync_typing_style_to_cursor();
                }
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
            ui.label(
                RichText::new("Character formatting and undo/redo are active. Save as .wordmock to keep formatting.")
                    .size(11.0)
                    .color(MUTED),
            );
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
            let (page_rect, _) =
                ui.allocate_exact_size(Vec2::new(page_width, page_height), Sense::hover());
            ui.painter().rect_filled(
                page_rect.translate(Vec2::new(0.0, 2.0)),
                2.0,
                Color32::from_black_alpha(18),
            );
            ui.painter().rect_filled(page_rect, 1.0, Color32::WHITE);
            ui.painter().rect_stroke(
                page_rect,
                1.0,
                Stroke::new(1.0, Color32::from_gray(222)),
                egui::StrokeKind::Inside,
            );

            let margin_x = 76.0 * self.zoom;
            let margin_y = 64.0 * self.zoom;
            let editor_rect = egui::Rect::from_min_max(
                page_rect.min + Vec2::new(margin_x, margin_y),
                page_rect.max - Vec2::new(margin_x, margin_y),
            );
            let mut editor_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(editor_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            let previous_text = self.document.text.clone();
            let formats = self.document.formats.clone();
            let font_size = self.document.font_size * self.zoom;
            let bold_font_available = self.bold_font_available;
            let mut layouter = move |ui: &egui::Ui, buffer: &dyn TextBuffer, wrap_width: f32| {
                let job = document_layout(
                    buffer.as_str(),
                    &formats,
                    font_size,
                    bold_font_available,
                    wrap_width,
                );
                ui.fonts_mut(|fonts| fonts.layout_job(job))
            };
            let edit = TextEdit::multiline(&mut self.document.text)
                .font(FontId::proportional(self.document.font_size * self.zoom))
                .id_salt(EDITOR_ID)
                .desired_width(editor_rect.width())
                .desired_rows(1)
                .min_size(editor_rect.size())
                .hint_text("Start writing…")
                .frame(egui::Frame::NONE)
                .margin(Vec2::ZERO)
                .layouter(&mut layouter);
            let output = edit.show(&mut editor_ui);
            if output.response.changed() {
                self.document
                    .reconcile_editor_change(&previous_text, self.typing_style);
            }
            if let Some(cursor_range) = output.cursor_range {
                let sorted = cursor_range.as_sorted_char_range();
                let next_range = usize::from(sorted.start)..usize::from(sorted.end);
                self.cursor_char = cursor_range.primary.index.into();
                if Some(next_range.clone()) != self.selection_chars || output.response.changed() {
                    self.sync_typing_style_to_cursor_range(&next_range);
                }
                self.selection_chars = Some(next_range);
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!(
                    "Page {} of {}",
                    self.document.page_count(),
                    self.document.page_count()
                ))
                .size(11.0)
                .color(MUTED),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("{} words", self.document.word_count()))
                    .size(11.0)
                    .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("{}%", (self.zoom * 100.0).round() as u32))
                        .size(11.0)
                        .color(MUTED),
                );
                ui.add(
                    egui::Slider::new(&mut self.zoom, 0.75..=1.35)
                        .show_value(false)
                        .step_by(0.05),
                );
                ui.label(RichText::new("−").color(MUTED));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("English (United States)")
                        .size(11.0)
                        .color(MUTED),
                );
                ui.add_space(12.0);
            });
        });
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);
        let command = modifiers.command;
        let (new, open, save, undo, redo, bold, italic, underline) = ctx.input(|i| {
            (
                i.key_pressed(Key::N) && command,
                i.key_pressed(Key::O) && command,
                i.key_pressed(Key::S) && command,
                i.key_pressed(Key::Z) && command && !i.modifiers.shift,
                (i.key_pressed(Key::Z) && command && i.modifiers.shift)
                    || (i.key_pressed(Key::Y) && command),
                i.key_pressed(Key::B) && command,
                i.key_pressed(Key::I) && command,
                i.key_pressed(Key::U) && command,
            )
        });
        if undo {
            ctx.input_mut(|i| {
                i.consume_key(modifiers, Key::Z);
            });
            self.document.undo();
            self.sync_typing_style_to_cursor();
        } else if redo {
            let key = if modifiers.shift { Key::Z } else { Key::Y };
            ctx.input_mut(|i| {
                i.consume_key(modifiers, key);
            });
            self.document.redo();
            self.sync_typing_style_to_cursor();
        } else if bold || italic || underline {
            let key = if bold {
                Key::B
            } else if italic {
                Key::I
            } else {
                Key::U
            };
            ctx.input_mut(|i| {
                i.consume_key(modifiers, key);
            });
            self.toggle_style(if bold {
                StyleKind::Bold
            } else if italic {
                StyleKind::Italic
            } else {
                StyleKind::Underline
            });
        } else if new {
            ctx.input_mut(|i| {
                i.consume_key(modifiers, Key::N);
            });
            self.new_document();
        } else if open {
            ctx.input_mut(|i| {
                i.consume_key(modifiers, Key::O);
            });
            self.open_document();
        } else if save {
            ctx.input_mut(|i| {
                i.consume_key(modifiers, Key::S);
            });
            self.save_document();
        }
    }

    fn toggle_style(&mut self, kind: StyleKind) {
        if let Some(range) = self.selected_byte_range()
            && !range.is_empty()
        {
            let enabled = !self.document.has_style(range.clone(), kind);
            self.document.toggle_style(range, kind);
            self.typing_style.set(kind, enabled);
        } else {
            self.typing_style.toggle(kind);
        }
    }

    fn selected_byte_range(&self) -> Option<Range<usize>> {
        self.selection_chars
            .as_ref()
            .map(|range| {
                char_to_byte(&self.document.text, range.start)
                    ..char_to_byte(&self.document.text, range.end)
            })
            .filter(|range| !range.is_empty())
    }

    fn sync_typing_style_to_cursor(&mut self) {
        if let Some(range) = self.selection_chars.clone() {
            self.sync_typing_style_to_cursor_range(&range);
        } else {
            self.typing_style = CharacterStyle::default();
        }
    }

    fn sync_typing_style_to_cursor_range(&mut self, range: &Range<usize>) {
        let cursor = self.cursor_char.min(self.document.text.chars().count());
        let cursor_byte = char_to_byte(&self.document.text, cursor);
        let style_byte = if cursor_byte > 0 {
            previous_char_boundary(&self.document.text, cursor_byte)
        } else {
            cursor_byte
        };
        self.typing_style = self.document.style_at(style_byte);
        self.selection_chars = Some(range.clone());
    }

    fn new_document(&mut self) {
        self.request_action(PendingAction::New);
    }

    fn open_document(&mut self) {
        self.request_action(PendingAction::Open);
    }

    fn pick_open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WordMock and text documents", &["wordmock", "txt", "md"])
            .pick_file()
        {
            match Document::open(path) {
                Ok(document) => {
                    self.document = document;
                    self.selection_chars = None;
                    self.cursor_char = 0;
                    self.typing_style = CharacterStyle::default();
                }
                Err(error) => self.message = Some(format!("Could not open file: {error}")),
            }
        }
    }

    fn save_document(&mut self) -> bool {
        if self.document.path.is_some() {
            let loses_formatting = !self.document.formats.is_empty()
                && !self.document.path.as_deref().is_some_and(is_wordmock_file);
            return match self.document.save() {
                Ok(()) => {
                    self.message = Some(if loses_formatting {
                        "Saved as plain text. Character formatting is kept only in .wordmock files."
                            .to_owned()
                    } else {
                        "Document saved.".to_owned()
                    });
                    true
                }
                Err(error) => {
                    self.message = Some(format!("Could not save file: {error}"));
                    false
                }
            };
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WordMock document", &["wordmock"])
            .add_filter("Plain text", &["txt", "md"])
            .set_file_name("Untitled.wordmock")
            .save_file()
        {
            return self.write_to(path);
        }
        false
    }

    fn write_to(&mut self, path: PathBuf) -> bool {
        let loses_formatting = !self.document.formats.is_empty() && !is_wordmock_file(&path);
        match self.document.save_as(path) {
            Ok(()) => {
                self.message = Some(if loses_formatting {
                    "Saved as plain text. Character formatting is kept only in .wordmock files."
                        .to_owned()
                } else {
                    "Document saved.".to_owned()
                });
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
            PendingAction::New => {
                self.document.reset();
                self.selection_chars = None;
                self.cursor_char = 0;
                self.typing_style = CharacterStyle::default();
            }
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
        let before = self.document.text.clone();
        if !self.document.text.is_empty() && !self.document.text.ends_with('\n') {
            self.document.text.push('\n');
        }
        self.document.text.push_str(prefix);
        self.document
            .reconcile_editor_change(&before, self.typing_style);
        self.cursor_char = self.document.text.chars().count();
        self.selection_chars = Some(self.cursor_char..self.cursor_char);
    }
}

fn install_bold_font(ctx: &egui::Context) -> bool {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/Library/Fonts/Arial Bold.ttf",
        ]
    } else if cfg!(target_os = "windows") {
        vec!["C:\\Windows\\Fonts\\arialbd.ttf"]
    } else {
        vec![
            "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        ]
    };

    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return false;
    };
    ctx.add_font(FontInsert::new(
        "WordMock-Bold-Font",
        FontData::from_owned(bytes),
        vec![InsertFontFamily {
            family: FontFamily::Name(Arc::from(BOLD_FONT_FAMILY)),
            priority: FontPriority::Highest,
        }],
    ));
    true
}

fn document_layout(
    text: &str,
    formats: &[crate::document::FormatSpan],
    font_size: f32,
    bold_font_available: bool,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.keep_trailing_whitespace = true;
    let mut cursor = 0;

    for span in formats {
        let start = floor_char_boundary(text, span.start.min(text.len())).max(cursor);
        let end = ceil_char_boundary(text, span.end.min(text.len()));
        if start >= end {
            continue;
        }
        if cursor < start {
            job.append(
                &text[cursor..start],
                0.0,
                text_format(CharacterStyle::default(), font_size, bold_font_available),
            );
        }
        job.append(
            &text[start..end],
            0.0,
            text_format(span.style, font_size, bold_font_available),
        );
        cursor = end;
    }
    if cursor < text.len() {
        job.append(
            &text[cursor..],
            0.0,
            text_format(CharacterStyle::default(), font_size, bold_font_available),
        );
    }
    job
}

fn text_format(style: CharacterStyle, size: f32, bold_font_available: bool) -> TextFormat {
    let family = if style.bold && bold_font_available {
        FontFamily::Name(Arc::from(BOLD_FONT_FAMILY))
    } else {
        FontFamily::Proportional
    };
    TextFormat {
        font_id: FontId::new(size, family),
        color: Color32::from_gray(30),
        italics: style.italic,
        underline: if style.underline {
            Stroke::new(1.0, Color32::from_gray(30))
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

fn is_wordmock_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wordmock"))
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte, _)| byte)
}

fn previous_char_boundary(text: &str, byte: usize) -> usize {
    text[..byte]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn floor_char_boundary(text: &str, byte: usize) -> usize {
    (0..=byte)
        .rev()
        .find(|offset| text.is_char_boundary(*offset))
        .unwrap_or(0)
}

fn ceil_char_boundary(text: &str, byte: usize) -> usize {
    (byte..=text.len())
        .find(|offset| text.is_char_boundary(*offset))
        .unwrap_or(text.len())
}
