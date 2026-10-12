use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use eframe::egui::TextBuffer;
use eframe::egui::epaint::text::VariationCoords;
use eframe::egui::text::{FontData, LayoutJob, TextFormat};
use eframe::egui::{
    self, Align, Color32, FontDefinitions, FontFamily, FontId, Key, Layout, RichText, Sense,
    Stroke, TextEdit, Vec2,
};

use crate::document::{
    CharacterStyle, Document, PageSize, ParagraphAlignment, StyleKind, supported_font_family,
};

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
    show_link_dialog: bool,
    link_url: String,
    link_label: String,
    show_find_dialog: bool,
    find_text: String,
    replace_text: String,
    pending_selection: Option<Range<usize>>,
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
            show_link_dialog: false,
            link_url: String::new(),
            link_label: String::new(),
            show_find_dialog: false,
            find_text: String::new(),
            replace_text: String::new(),
            pending_selection: None,
        }
    }
}

impl WordMockApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut app = Self::default();
        app.bold_font_available = install_fonts(ctx);
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
        self.hyperlink_dialog(&ctx);
        self.find_replace_dialog(&ctx);

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
            RibbonTab::Insert => self.insert_ribbon(ui),
            RibbonTab::Layout => self.layout_ribbon(ui),
            RibbonTab::Review => self.review_ribbon(ui),
            RibbonTab::View => self.view_ribbon(ui),
        });
    }

    fn home_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let current_font = self
                    .typing_style
                    .font_family
                    .as_deref()
                    .unwrap_or(&self.document.font_family)
                    .to_owned();
                egui::ComboBox::from_id_salt("wordmock-font-family")
                    .selected_text(current_font)
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for family in ["Roboto", "Open Sans", "Lora", "Roboto Mono", "System"] {
                            if ui
                                .selectable_label(self.active_font_family() == family, family)
                                .clicked()
                            {
                                self.set_font_family(family);
                            }
                        }
                    });

                let mut font_size = self
                    .typing_style
                    .font_size
                    .unwrap_or(self.document.font_size);
                if ui
                    .add(egui::Slider::new(&mut font_size, 8.0..=72.0).suffix(" pt"))
                    .changed()
                {
                    self.set_font_size(font_size);
                }

                let mut text_color = color_from_style(self.typing_style.color, Color32::BLACK);
                if ui.color_edit_button_srgba(&mut text_color).changed() {
                    self.set_text_color(text_color);
                }
                ui.label(RichText::new("A").color(text_color).strong());
                if ui
                    .button("🖍")
                    .on_hover_text("Apply yellow highlight")
                    .clicked()
                {
                    self.set_highlight(Some(Color32::from_rgb(255, 236, 130)));
                }
                if ui.button("No highlight").clicked() {
                    self.set_highlight(None);
                }
                if ui
                    .button("Clear")
                    .on_hover_text("Clear character formatting")
                    .clicked()
                {
                    self.clear_formatting();
                }
            });
            ui.horizontal(|ui| {
                let selected = self.selected_byte_range();
                let bold = selected.as_ref().map_or(self.typing_style.bold, |range| {
                    self.document.has_style(range.clone(), StyleKind::Bold)
                });
                let italic = selected.as_ref().map_or(self.typing_style.italic, |range| {
                    self.document.has_style(range.clone(), StyleKind::Italic)
                });
                let underline = selected
                    .as_ref()
                    .map_or(self.typing_style.underline, |range| {
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
                if ui
                    .add_enabled(self.document.can_undo(), egui::Button::new("↶ Undo"))
                    .clicked()
                {
                    self.document.undo();
                    self.sync_typing_style_to_cursor();
                }
                if ui
                    .add_enabled(self.document.can_redo(), egui::Button::new("↷ Redo"))
                    .clicked()
                {
                    self.document.redo();
                    self.sync_typing_style_to_cursor();
                }
                ui.separator();
                for (alignment, title) in [
                    (ParagraphAlignment::Left, "Left"),
                    (ParagraphAlignment::Center, "Center"),
                    (ParagraphAlignment::Right, "Right"),
                    (ParagraphAlignment::Justify, "Justify"),
                ] {
                    if ui
                        .selectable_label(self.document.alignment == alignment, title)
                        .clicked()
                    {
                        self.document.set_alignment(alignment);
                    }
                }
                ui.separator();
                if ui.button("• Bullets").clicked() {
                    self.insert_paragraph_prefix("• ");
                }
                if ui.button("1. Numbering").clicked() {
                    self.insert_paragraph_prefix("1. ");
                }
                ui.separator();
                egui::ComboBox::from_id_salt("wordmock-line-spacing")
                    .selected_text(format!("{}× spacing", self.document.line_spacing))
                    .show_ui(ui, |ui| {
                        for spacing in [1.0_f32, 1.15, 1.5, 2.0, 2.5] {
                            if ui
                                .selectable_label(
                                    (self.document.line_spacing - spacing).abs() < 0.01,
                                    format!("{spacing}×"),
                                )
                                .clicked()
                            {
                                self.document.set_line_spacing(spacing);
                            }
                        }
                    });
            });
        });
    }

    fn insert_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        for (label, value) in [
            ("Date", Local::now().format("%B %-d, %Y").to_string()),
            ("Time", Local::now().format("%-I:%M %p").to_string()),
        ] {
            if ui.button(label).clicked() {
                self.insert_text_at_selection(&value, self.typing_style.clone());
            }
        }
        if ui.button("Hyperlink").clicked() {
            self.link_label = self.selected_text().unwrap_or_default();
            self.link_url.clear();
            self.show_link_dialog = true;
        }
        if ui.button("Page break marker").clicked() {
            self.insert_text_at_selection(
                "\n\n──────────────── Page Break ────────────────\n\n",
                CharacterStyle::default(),
            );
        }
        if ui.button("3 × 3 text table").clicked() {
            self.insert_text_at_selection(
                "Column 1\tColumn 2\tColumn 3\nCell\tCell\tCell\nCell\tCell\tCell\n",
                CharacterStyle::default(),
            );
        }
        ui.label(
            RichText::new("Images and true grid tables need the upcoming object model.")
                .size(11.0)
                .color(MUTED),
        );
    }

    fn layout_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.label("Paper");
        egui::ComboBox::from_id_salt("wordmock-paper-size")
            .selected_text(format!("{:?}", self.document.page_size))
            .show_ui(ui, |ui| {
                for (size, label) in [
                    (PageSize::Letter, "Letter"),
                    (PageSize::A4, "A4"),
                    (PageSize::Legal, "Legal"),
                ] {
                    if ui
                        .selectable_label(self.document.page_size == size, label)
                        .clicked()
                    {
                        self.document.set_page_size(size);
                    }
                }
            });
        if ui
            .button(if self.document.landscape {
                "Landscape"
            } else {
                "Portrait"
            })
            .clicked()
        {
            self.document.set_landscape(!self.document.landscape);
        }
        ui.separator();
        ui.label("Margins");
        let mut margin = self.document.margin;
        if ui
            .add(egui::Slider::new(&mut margin, 24.0..=120.0).suffix(" pt"))
            .changed()
        {
            self.document.set_margin(margin);
        }
    }

    fn review_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        if ui.button("Find and replace…").clicked() {
            self.show_find_dialog = true;
        }
        ui.separator();
        ui.label(format!("{} words", self.document.word_count()));
        ui.label(format!("{} characters", self.document.text.chars().count()));
        ui.label(
            RichText::new("Spelling and tracked changes are planned after pagination.")
                .size(11.0)
                .color(MUTED),
        );
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
        let (paper_width, paper_height) = self.document.page_dimensions();
        let page_width = paper_width * self.zoom;
        let page_height = paper_height * self.zoom;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |scroll_ui| {
                scroll_ui.allocate_ui_with_layout(
                    Vec2::new((page_width + 32.0).max(available.x), page_height + 32.0),
                    Layout::top_down(Align::Center),
                    |ui| {
                        ui.add_space(16.0);
                        let (page_rect, _) = ui.allocate_exact_size(
                            Vec2::new(page_width, page_height),
                            Sense::hover(),
                        );
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

                        let margin = (self.document.margin * self.zoom)
                            .min(page_width * 0.4)
                            .min(page_height * 0.4);
                        let editor_rect = egui::Rect::from_min_max(
                            page_rect.min + Vec2::splat(margin),
                            page_rect.max - Vec2::splat(margin),
                        );
                        let mut editor_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(editor_rect)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        let previous_text = self.document.text.clone();
                        let formats = self.document.formats.clone();
                        let font_size = self.document.font_size * self.zoom;
                        let font_family = self.document.font_family.clone();
                        let line_spacing = self.document.line_spacing;
                        let alignment = self.document.alignment;
                        let bold_font_available = self.bold_font_available;
                        let mut layouter =
                            move |ui: &egui::Ui, buffer: &dyn TextBuffer, wrap_width: f32| {
                                let job = document_layout(
                                    buffer.as_str(),
                                    &formats,
                                    &font_family,
                                    font_size,
                                    line_spacing,
                                    alignment,
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
                        let mut output = edit.show(&mut editor_ui);
                        if output.response.changed() {
                            self.document
                                .reconcile_editor_change(&previous_text, self.typing_style.clone());
                        }
                        if let Some(cursor_range) = output.cursor_range {
                            let sorted = cursor_range.as_sorted_char_range();
                            let next_range = usize::from(sorted.start)..usize::from(sorted.end);
                            self.cursor_char = cursor_range.primary.index.into();
                            if Some(next_range.clone()) != self.selection_chars
                                || output.response.changed()
                            {
                                self.sync_typing_style_to_cursor_range(&next_range);
                            }
                            self.selection_chars = Some(next_range);
                        }
                        if let Some(range) = self.pending_selection.take() {
                            output.state.cursor.set_char_range(Some(
                                egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(range.start),
                                    egui::text::CCursor::new(range.end),
                                ),
                            ));
                            output.state.store(ui.ctx(), output.response.id);
                            self.cursor_char = range.end;
                            self.selection_chars = Some(range);
                        }
                    },
                );
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
        let (new, open, save, find, undo, redo, bold, italic, underline) = ctx.input(|i| {
            (
                i.key_pressed(Key::N) && command,
                i.key_pressed(Key::O) && command,
                i.key_pressed(Key::S) && command,
                i.key_pressed(Key::F) && command,
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
        } else if find {
            ctx.input_mut(|i| {
                i.consume_key(modifiers, Key::F);
            });
            self.show_find_dialog = true;
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

    fn active_font_family(&self) -> &str {
        self.typing_style
            .font_family
            .as_deref()
            .unwrap_or(&self.document.font_family)
    }

    fn set_font_family(&mut self, family: &str) {
        if !supported_font_family(family) {
            return;
        }
        if let Some(range) = self.selected_byte_range() {
            let family = family.to_owned();
            self.document.update_character_style(range, |style| {
                style.font_family = Some(family.clone());
            });
            self.typing_style.font_family = Some(family);
        } else {
            self.typing_style.font_family = Some(family.to_owned());
            if self.document.text.is_empty() {
                self.document.set_font_family(family);
            }
        }
    }

    fn set_font_size(&mut self, size: f32) {
        if let Some(range) = self.selected_byte_range() {
            self.document.update_character_style(range, |style| {
                style.font_size = Some(size);
            });
            self.typing_style.font_size = Some(size);
        } else if self.document.text.is_empty() {
            self.document.set_font_size(size);
        } else {
            self.typing_style.font_size = Some(size);
        }
    }

    fn set_text_color(&mut self, color: Color32) {
        let rgba = color_to_rgba(color);
        if let Some(range) = self.selected_byte_range() {
            self.document.update_character_style(range, |style| {
                style.color = Some(rgba);
            });
        }
        self.typing_style.color = Some(rgba);
    }

    fn set_highlight(&mut self, highlight: Option<Color32>) {
        let rgba = highlight.map(color_to_rgba);
        if let Some(range) = self.selected_byte_range() {
            self.document.update_character_style(range, |style| {
                style.highlight = rgba;
            });
        }
        self.typing_style.highlight = rgba;
    }

    fn clear_formatting(&mut self) {
        if let Some(range) = self.selected_byte_range() {
            self.document.clear_character_style(range);
        }
        self.typing_style = CharacterStyle::default();
    }

    fn selected_text(&self) -> Option<String> {
        let range = self
            .selection_chars
            .as_ref()
            .filter(|range| range.start < range.end)?;
        Some(
            self.document
                .text
                .chars()
                .skip(range.start)
                .take(range.end - range.start)
                .collect(),
        )
    }

    fn insert_text_at_selection(&mut self, inserted: &str, style: CharacterStyle) {
        let char_range = self
            .selection_chars
            .clone()
            .filter(|range| range.start < range.end);
        let start_char = char_range
            .as_ref()
            .map_or(self.cursor_char, |range| range.start);
        let end_char = char_range
            .as_ref()
            .map_or(self.cursor_char, |range| range.end);
        let start_byte = char_to_byte(&self.document.text, start_char);
        let end_byte = char_to_byte(&self.document.text, end_char);
        let before = self.document.text.clone();
        self.document
            .text
            .replace_range(start_byte..end_byte, inserted);
        self.document.reconcile_editor_change(&before, style);
        let new_cursor = start_char + inserted.chars().count();
        self.cursor_char = new_cursor;
        self.selection_chars = Some(new_cursor..new_cursor);
    }

    fn insert_paragraph_prefix(&mut self, prefix: &str) {
        let cursor_byte = char_to_byte(&self.document.text, self.cursor_char);
        let paragraph_start = self.document.text[..cursor_byte]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let (remove_len, inserted) = if self.document.text[paragraph_start..].starts_with(prefix) {
            (prefix.len(), "")
        } else {
            (0, prefix)
        };
        let before = self.document.text.clone();
        self.document
            .text
            .replace_range(paragraph_start..paragraph_start + remove_len, inserted);
        self.document
            .reconcile_editor_change(&before, CharacterStyle::default());
        let new_cursor_byte = cursor_byte
            .saturating_sub(remove_len)
            .saturating_add(inserted.len());
        let new_cursor = self.document.text[..new_cursor_byte].chars().count();
        self.cursor_char = new_cursor;
        self.selection_chars = Some(new_cursor..new_cursor);
    }

    fn create_hyperlink(&mut self) {
        let url = self.link_url.trim();
        let has_supported_scheme = ["https://", "http://", "mailto:"].iter().any(|scheme| {
            url.strip_prefix(scheme)
                .is_some_and(|tail| !tail.is_empty())
        });
        if !has_supported_scheme || url.chars().any(char::is_whitespace) {
            self.message = Some("Enter a valid http, https, or mailto link.".to_owned());
            return;
        }
        let mut style = self.typing_style.clone();
        style.hyperlink = Some(url.to_owned());
        style.color = Some([28, 82, 168, 255]);
        style.underline = true;

        if let Some(range) = self.selected_byte_range() {
            let selected = self.selected_text().unwrap_or_default();
            if self.link_label.trim().is_empty() || self.link_label == selected {
                self.document.update_character_style(range, |current| {
                    current.hyperlink = style.hyperlink.clone();
                    current.color = style.color;
                    current.underline = true;
                });
                self.typing_style = style;
            } else {
                let label = self.link_label.trim().to_owned();
                self.insert_text_at_selection(&label, style);
            }
        } else {
            let label = if self.link_label.trim().is_empty() {
                url.to_owned()
            } else {
                self.link_label.trim().to_owned()
            };
            self.insert_text_at_selection(&label, style);
        }
        self.show_link_dialog = false;
    }

    fn find_next(&mut self) {
        if self.find_text.is_empty() {
            return;
        }
        let matches: Vec<(usize, usize)> = self
            .document
            .text
            .match_indices(&self.find_text)
            .map(|(start, matched)| (start, start + matched.len()))
            .collect();
        if matches.is_empty() {
            self.message = Some("No matches found.".to_owned());
            return;
        }
        let cursor_byte = char_to_byte(&self.document.text, self.cursor_char);
        let (start, end) = matches
            .iter()
            .copied()
            .find(|(start, _)| *start >= cursor_byte)
            .unwrap_or(matches[0]);
        let char_start = self.document.text[..start].chars().count();
        let char_end = self.document.text[..end].chars().count();
        self.pending_selection = Some(char_start..char_end);
        self.selection_chars = Some(char_start..char_end);
        self.cursor_char = char_end;
    }

    fn replace_current(&mut self) {
        if self.find_text.is_empty() {
            return;
        }
        if self
            .selected_text()
            .is_some_and(|selected| selected == self.find_text)
        {
            let replacement = self.replace_text.clone();
            let style = self
                .selected_byte_range()
                .map_or_else(CharacterStyle::default, |range| {
                    self.document.style_at(range.start)
                });
            self.insert_text_at_selection(&replacement, style);
        } else {
            self.find_next();
        }
    }

    fn replace_all(&mut self) {
        if self.find_text.is_empty() {
            return;
        }
        let count = self
            .document
            .replace_all(&self.find_text, &self.replace_text);
        self.cursor_char = self.cursor_char.min(self.document.text.chars().count());
        self.selection_chars = None;
        self.pending_selection = None;
        self.message = Some(format!("Replaced {count} occurrence(s)."));
    }

    fn hyperlink_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_link_dialog {
            return;
        }
        let mut create = false;
        let mut cancel = false;
        egui::Window::new("Insert hyperlink")
            .collapsible(false)
            .resizable(false)
            .open(&mut self.show_link_dialog)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Display text");
                ui.text_edit_singleline(&mut self.link_label);
                ui.label("Address");
                ui.text_edit_singleline(&mut self.link_url);
                ui.horizontal(|ui| {
                    if ui.button("Insert").clicked() {
                        create = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if create {
            self.create_hyperlink();
        } else if cancel {
            self.show_link_dialog = false;
        }
    }

    fn find_replace_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_find_dialog {
            return;
        }
        let mut action = None;
        egui::Window::new("Find and replace")
            .collapsible(false)
            .resizable(false)
            .open(&mut self.show_find_dialog)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Find");
                ui.text_edit_singleline(&mut self.find_text);
                ui.label("Replace with");
                ui.text_edit_singleline(&mut self.replace_text);
                ui.horizontal(|ui| {
                    if ui.button("Find next").clicked() {
                        action = Some(0);
                    }
                    if ui.button("Replace").clicked() {
                        action = Some(1);
                    }
                    if ui.button("Replace all").clicked() {
                        action = Some(2);
                    }
                });
            });
        match action {
            Some(0) => self.find_next(),
            Some(1) => self.replace_current(),
            Some(2) => self.replace_all(),
            _ => {}
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
            let loses_formatting = self.document_has_non_text_settings()
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
        let loses_formatting = self.document_has_non_text_settings() && !is_wordmock_file(&path);
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

    fn document_has_non_text_settings(&self) -> bool {
        !self.document.formats.is_empty()
            || (self.document.font_size - 12.0).abs() > f32::EPSILON
            || self.document.font_family != "Roboto"
            || self.document.page_size != PageSize::Letter
            || self.document.landscape
            || (self.document.margin - 76.0).abs() > f32::EPSILON
            || (self.document.line_spacing - 1.0).abs() > f32::EPSILON
            || self.document.alignment != ParagraphAlignment::Left
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
}

fn install_fonts(ctx: &egui::Context) -> bool {
    // The app creator runs before the first egui frame, so Context::fonts is
    // not initialized yet. Extend the standard definitions and install them.
    let mut definitions = FontDefinitions::default();
    let fallbacks = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    add_variable_font(
        &mut definitions,
        "Roboto",
        include_bytes!("../assets/fonts/Roboto.ttf"),
        &fallbacks,
    );
    add_variable_font(
        &mut definitions,
        "Open Sans",
        include_bytes!("../assets/fonts/OpenSans.ttf"),
        &fallbacks,
    );
    add_variable_font(
        &mut definitions,
        "Lora",
        include_bytes!("../assets/fonts/Lora.ttf"),
        &fallbacks,
    );
    add_variable_font(
        &mut definitions,
        "Roboto Mono",
        include_bytes!("../assets/fonts/RobotoMono.ttf"),
        &fallbacks,
    );

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

    let system_bold = candidates.iter().find_map(|path| std::fs::read(path).ok());
    let bold_available = system_bold.is_some();
    if let Some(bytes) = system_bold {
        let font_name = "WordMock-System-Bold";
        definitions
            .font_data
            .insert(font_name.to_owned(), Arc::new(FontData::from_owned(bytes)));
        let mut family = vec![font_name.to_owned()];
        family.extend(fallbacks);
        definitions
            .families
            .insert(FontFamily::Name(Arc::from(BOLD_FONT_FAMILY)), family);
    }
    ctx.set_fonts(definitions);
    bold_available
}

fn add_variable_font(
    definitions: &mut FontDefinitions,
    family_name: &str,
    bytes: &'static [u8],
    fallbacks: &[String],
) {
    let font_name = format!("WordMock-{family_name}");
    definitions
        .font_data
        .insert(font_name.clone(), Arc::new(FontData::from_static(bytes)));
    let mut family = vec![font_name];
    family.extend_from_slice(fallbacks);
    definitions
        .families
        .insert(FontFamily::Name(Arc::from(family_name)), family);
}

fn document_layout(
    text: &str,
    formats: &[crate::document::FormatSpan],
    base_font_family: &str,
    font_size: f32,
    line_spacing: f32,
    alignment: ParagraphAlignment,
    bold_font_available: bool,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.keep_trailing_whitespace = true;
    job.halign = match alignment {
        ParagraphAlignment::Left | ParagraphAlignment::Justify => Align::LEFT,
        ParagraphAlignment::Center => Align::Center,
        ParagraphAlignment::Right => Align::RIGHT,
    };
    job.justify = alignment == ParagraphAlignment::Justify;
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
                text_format(
                    &CharacterStyle::default(),
                    base_font_family,
                    font_size,
                    line_spacing,
                    bold_font_available,
                ),
            );
        }
        job.append(
            &text[start..end],
            0.0,
            text_format(
                &span.style,
                base_font_family,
                font_size,
                line_spacing,
                bold_font_available,
            ),
        );
        cursor = end;
    }
    if cursor < text.len() {
        job.append(
            &text[cursor..],
            0.0,
            text_format(
                &CharacterStyle::default(),
                base_font_family,
                font_size,
                line_spacing,
                bold_font_available,
            ),
        );
    }
    job
}

fn text_format(
    style: &CharacterStyle,
    base_font_family: &str,
    size: f32,
    line_spacing: f32,
    bold_font_available: bool,
) -> TextFormat {
    let font_family = style.font_family.as_deref().unwrap_or(base_font_family);
    let use_system_bold = style.bold && font_family == "System" && bold_font_available;
    let resolved_family = if style.bold && font_family == "System" && !bold_font_available {
        "Roboto"
    } else {
        font_family
    };
    let family = if use_system_bold {
        FontFamily::Name(Arc::from(BOLD_FONT_FAMILY))
    } else if supported_font_family(resolved_family) && resolved_family != "System" {
        FontFamily::Name(Arc::from(resolved_family))
    } else {
        FontFamily::Proportional
    };
    let font_size = style.font_size.unwrap_or(size);
    let default_color = if style.hyperlink.is_some() {
        Color32::from_rgb(28, 82, 168)
    } else {
        Color32::from_gray(30)
    };
    let color = style.color.map_or(default_color, color_from_rgba);
    let coords = if resolved_family != "System" {
        VariationCoords::new([("wght", if style.bold { 700.0 } else { 400.0 })])
    } else {
        VariationCoords::default()
    };
    TextFormat {
        font_id: FontId::new(font_size, family),
        color,
        background: style
            .highlight
            .map_or(Color32::TRANSPARENT, color_from_rgba),
        coords,
        italics: style.italic,
        underline: if style.underline || style.hyperlink.is_some() {
            Stroke::new(1.0, color)
        } else {
            Stroke::NONE
        },
        line_height: Some(font_size * 1.25 * line_spacing),
        ..Default::default()
    }
}

fn color_from_rgba(color: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

fn color_to_rgba(color: Color32) -> [u8; 4] {
    [color.r(), color.g(), color.b(), color.a()]
}

fn color_from_style(color: Option<[u8; 4]>, default: Color32) -> Color32 {
    color.map_or(default, color_from_rgba)
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
