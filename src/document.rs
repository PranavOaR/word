use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const MAX_UNDO_STEPS: usize = 100;
const TYPE_GROUP_DELAY: Duration = Duration::from_millis(700);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub color: Option<[u8; 4]>,
    pub highlight: Option<[u8; 4]>,
    pub hyperlink: Option<String>,
}

impl CharacterStyle {
    pub fn is_plain(&self) -> bool {
        !self.bold
            && !self.italic
            && !self.underline
            && self.font_family.is_none()
            && self.font_size.is_none()
            && self.color.is_none()
            && self.highlight.is_none()
            && self.hyperlink.is_none()
    }

    pub fn toggle(&mut self, kind: StyleKind) {
        match kind {
            StyleKind::Bold => self.bold = !self.bold,
            StyleKind::Italic => self.italic = !self.italic,
            StyleKind::Underline => self.underline = !self.underline,
        }
    }
}

#[derive(Clone, Copy)]
pub enum StyleKind {
    Bold,
    Italic,
    Underline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormatSpan {
    /// UTF-8 byte offsets into the document text.
    pub start: usize,
    pub end: usize,
    pub style: CharacterStyle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSize {
    #[default]
    Letter,
    A4,
    Legal,
}

impl PageSize {
    pub fn dimensions(self) -> (f32, f32) {
        match self {
            Self::Letter => (816.0, 1056.0),
            Self::A4 => (794.0, 1123.0),
            Self::Legal => (816.0, 1344.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug)]
pub struct Document {
    pub text: String,
    pub formats: Vec<FormatSpan>,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub font_size: f32,
    pub font_family: String,
    pub page_size: PageSize,
    pub landscape: bool,
    pub margin: f32,
    pub line_spacing: f32,
    pub alignment: ParagraphAlignment,
    undo_stack: Vec<DocumentSnapshot>,
    redo_stack: Vec<DocumentSnapshot>,
    last_text_edit: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
struct DocumentSnapshot {
    text: String,
    formats: Vec<FormatSpan>,
    font_size: f32,
    font_family: String,
    page_size: PageSize,
    landscape: bool,
    margin: f32,
    line_spacing: f32,
    alignment: ParagraphAlignment,
}

#[derive(Serialize, Deserialize)]
struct SavedDocument {
    format_version: u32,
    text: String,
    formats: Vec<FormatSpan>,
    font_size: f32,
    #[serde(default = "default_font_family")]
    font_family: String,
    #[serde(default)]
    page_size: PageSize,
    #[serde(default)]
    landscape: bool,
    #[serde(default = "default_margin")]
    margin: f32,
    #[serde(default = "default_line_spacing")]
    line_spacing: f32,
    #[serde(default)]
    alignment: ParagraphAlignment,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: String::new(),
            formats: Vec::new(),
            path: None,
            dirty: false,
            font_size: 12.0,
            font_family: default_font_family(),
            page_size: PageSize::Letter,
            landscape: false,
            margin: 76.0,
            line_spacing: 1.0,
            alignment: ParagraphAlignment::Left,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_text_edit: None,
        }
    }
}

impl Document {
    pub fn title(&self) -> &str {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled document")
    }

    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    pub fn page_count(&self) -> usize {
        // A rough estimate for the status bar until pagination is implemented.
        (self.text.chars().count().max(1) + 1_799) / 1_800
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn has_style(&self, bytes: Range<usize>, kind: StyleKind) -> bool {
        if bytes.is_empty() {
            return self.style_at(bytes.start).style(kind);
        }
        let mut covered = bytes.start;
        for span in &self.formats {
            if span.end <= covered || span.start >= bytes.end {
                continue;
            }
            if span.start > covered || !span.style.style(kind) {
                return false;
            }
            covered = span.end.min(bytes.end);
            if covered == bytes.end {
                return true;
            }
        }
        false
    }

    pub fn style_at(&self, byte: usize) -> CharacterStyle {
        self.formats
            .iter()
            .find(|span| span.start <= byte && byte < span.end)
            .map_or_else(CharacterStyle::default, |span| span.style.clone())
    }

    pub fn toggle_style(&mut self, bytes: Range<usize>, kind: StyleKind) {
        let range = clamp_range_to_char_boundaries(&self.text, bytes);
        if range.is_empty() {
            return;
        }

        let disable = self.has_style(range.clone(), kind);
        self.update_character_style(range, |style| style.set(kind, !disable));
    }

    pub fn update_character_style(
        &mut self,
        bytes: Range<usize>,
        mut update: impl FnMut(&mut CharacterStyle),
    ) {
        let range = clamp_range_to_char_boundaries(&self.text, bytes);
        if range.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        let mut runs = self.style_runs();
        let mut edited = Vec::with_capacity(runs.len() + 2);
        for (start, end, mut style) in runs.drain(..) {
            if end <= range.start || start >= range.end {
                edited.push((start, end, style));
                continue;
            }
            let selected_start = start.max(range.start);
            let selected_end = end.min(range.end);
            if start < selected_start {
                edited.push((start, selected_start, style.clone()));
            }
            let right_style = style.clone();
            update(&mut style);
            edited.push((selected_start, selected_end, style));
            if selected_end < end {
                edited.push((selected_end, end, right_style));
            }
        }
        self.formats = spans_from_runs(edited);
        self.mark_changed();
    }

    pub fn clear_character_style(&mut self, bytes: Range<usize>) {
        self.update_character_style(bytes, |style| *style = CharacterStyle::default());
    }

    pub fn reconcile_editor_change(&mut self, before: &str, inserted_style: CharacterStyle) {
        if before == self.text {
            return;
        }

        let now = Instant::now();
        if self
            .last_text_edit
            .is_none_or(|last| now.duration_since(last) > TYPE_GROUP_DELAY)
        {
            self.push_undo_snapshot_with_text(before);
        }

        let (old_start, old_end, new_start, new_end) = changed_byte_ranges(before, &self.text);
        let delta =
            (new_end as isize - new_start as isize) - (old_end as isize - old_start as isize);
        let mut adjusted = Vec::new();

        for span in &self.formats {
            if span.start < old_start {
                let end = span.end.min(old_start);
                if span.start < end {
                    adjusted.push(FormatSpan {
                        start: span.start,
                        end,
                        style: span.style.clone(),
                    });
                }
            }
            if span.end > old_end {
                let start = span.start.max(old_end);
                if start < span.end {
                    adjusted.push(FormatSpan {
                        start: shift_byte(start, delta),
                        end: shift_byte(span.end, delta),
                        style: span.style.clone(),
                    });
                }
            }
        }

        if new_start < new_end && !inserted_style.is_plain() {
            adjusted.push(FormatSpan {
                start: new_start,
                end: new_end,
                style: inserted_style,
            });
        }

        adjusted.sort_by_key(|span| span.start);
        self.formats = merge_spans(adjusted);
        self.last_text_edit = Some(now);
        self.redo_stack.clear();
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(self.snapshot());
        self.restore(snapshot);
        self.last_text_edit = None;
        self.dirty = true;
    }

    pub fn redo(&mut self) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(self.snapshot());
        self.restore(snapshot);
        self.last_text_edit = None;
        self.dirty = true;
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        if (self.font_size - font_size).abs() < f32::EPSILON {
            return;
        }
        self.push_undo_snapshot();
        self.font_size = font_size;
        self.mark_changed();
    }

    pub fn set_font_family(&mut self, font_family: impl Into<String>) {
        let font_family = font_family.into();
        if self.font_family == font_family {
            return;
        }
        self.push_undo_snapshot();
        self.font_family = font_family;
        self.mark_changed();
    }

    pub fn set_page_size(&mut self, page_size: PageSize) {
        if self.page_size == page_size {
            return;
        }
        self.push_undo_snapshot();
        self.page_size = page_size;
        self.mark_changed();
    }

    pub fn set_landscape(&mut self, landscape: bool) {
        if self.landscape == landscape {
            return;
        }
        self.push_undo_snapshot();
        self.landscape = landscape;
        self.mark_changed();
    }

    pub fn set_margin(&mut self, margin: f32) {
        let margin = margin.clamp(24.0, 120.0);
        if (self.margin - margin).abs() < 0.25 {
            return;
        }
        self.push_undo_snapshot();
        self.margin = margin;
        self.mark_changed();
    }

    pub fn set_line_spacing(&mut self, line_spacing: f32) {
        let line_spacing = line_spacing.clamp(1.0, 2.5);
        if (self.line_spacing - line_spacing).abs() < 0.01 {
            return;
        }
        self.push_undo_snapshot();
        self.line_spacing = line_spacing;
        self.mark_changed();
    }

    pub fn set_alignment(&mut self, alignment: ParagraphAlignment) {
        if self.alignment == alignment {
            return;
        }
        self.push_undo_snapshot();
        self.alignment = alignment;
        self.mark_changed();
    }

    pub fn page_dimensions(&self) -> (f32, f32) {
        let (width, height) = self.page_size.dimensions();
        if self.landscape {
            (height, width)
        } else {
            (width, height)
        }
    }

    pub fn replace_all(&mut self, search: &str, replacement: &str) -> usize {
        if search.is_empty() {
            return 0;
        }
        let matches: Vec<usize> = self
            .text
            .match_indices(search)
            .map(|(byte, _)| byte)
            .collect();
        if matches.is_empty() {
            return 0;
        }

        self.last_text_edit = None;
        for byte in matches.iter().rev().copied() {
            let end = byte + search.len();
            let style = self.style_at(byte);
            let before = self.text.clone();
            self.text.replace_range(byte..end, replacement);
            self.reconcile_editor_change(&before, style);
        }
        matches.len()
    }

    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let (
            text,
            formats,
            font_size,
            font_family,
            page_size,
            landscape,
            margin,
            line_spacing,
            alignment,
        ) = if has_wordmock_extension(path) {
            let saved: SavedDocument = serde_json::from_slice(&fs::read(path)?)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if !matches!(saved.format_version, 1 | 2) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "unsupported WordMock document version {}",
                        saved.format_version
                    ),
                ));
            }
            validate_saved_document(&saved)?;
            (
                saved.text,
                saved.formats,
                saved.font_size,
                saved.font_family,
                saved.page_size,
                saved.landscape,
                saved.margin,
                saved.line_spacing,
                saved.alignment,
            )
        } else {
            (
                fs::read_to_string(path)?,
                Vec::new(),
                12.0,
                default_font_family(),
                PageSize::Letter,
                false,
                default_margin(),
                default_line_spacing(),
                ParagraphAlignment::Left,
            )
        };

        Ok(Self {
            text,
            formats,
            path: Some(path.to_path_buf()),
            dirty: false,
            font_size,
            font_family,
            page_size,
            landscape,
            margin,
            line_spacing,
            alignment,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_text_edit: None,
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.as_deref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "document has no file path")
        })?;
        self.write(path)?;
        self.dirty = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        self.write(path)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn write(&self, path: &Path) -> io::Result<()> {
        if has_wordmock_extension(path) {
            let saved = SavedDocument {
                format_version: 2,
                text: self.text.clone(),
                formats: self.formats.clone(),
                font_size: self.font_size,
                font_family: self.font_family.clone(),
                page_size: self.page_size,
                landscape: self.landscape,
                margin: self.margin,
                line_spacing: self.line_spacing,
                alignment: self.alignment,
            };
            let bytes = serde_json::to_vec_pretty(&saved)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            fs::write(path, bytes)
        } else {
            fs::write(path, &self.text)
        }
    }

    fn style_runs(&self) -> Vec<(usize, usize, CharacterStyle)> {
        let mut runs = Vec::with_capacity(self.formats.len() * 2 + 1);
        let mut cursor = 0;
        for span in &self.formats {
            if cursor < span.start {
                runs.push((cursor, span.start, CharacterStyle::default()));
            }
            runs.push((span.start, span.end, span.style.clone()));
            cursor = span.end;
        }
        if cursor < self.text.len() {
            runs.push((cursor, self.text.len(), CharacterStyle::default()));
        }
        runs
    }

    fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            text: self.text.clone(),
            formats: self.formats.clone(),
            font_size: self.font_size,
            font_family: self.font_family.clone(),
            page_size: self.page_size,
            landscape: self.landscape,
            margin: self.margin,
            line_spacing: self.line_spacing,
            alignment: self.alignment,
        }
    }

    fn push_undo_snapshot(&mut self) {
        let snapshot = self.snapshot();
        self.push_undo(snapshot);
    }

    fn push_undo_snapshot_with_text(&mut self, previous_text: &str) {
        let mut snapshot = self.snapshot();
        snapshot.text = previous_text.to_owned();
        // The previous formatting is already represented by the current spans before
        // the edit is reconciled; this path is called before those spans are changed.
        self.push_undo(snapshot);
    }

    fn push_undo(&mut self, snapshot: DocumentSnapshot) {
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn restore(&mut self, snapshot: DocumentSnapshot) {
        self.text = snapshot.text;
        self.formats = snapshot.formats;
        self.font_size = snapshot.font_size;
        self.font_family = snapshot.font_family;
        self.page_size = snapshot.page_size;
        self.landscape = snapshot.landscape;
        self.margin = snapshot.margin;
        self.line_spacing = snapshot.line_spacing;
        self.alignment = snapshot.alignment;
    }

    fn mark_changed(&mut self) {
        self.dirty = true;
        self.redo_stack.clear();
        self.last_text_edit = None;
    }
}

impl CharacterStyle {
    fn style(&self, kind: StyleKind) -> bool {
        match kind {
            StyleKind::Bold => self.bold,
            StyleKind::Italic => self.italic,
            StyleKind::Underline => self.underline,
        }
    }

    pub fn set(&mut self, kind: StyleKind, enabled: bool) {
        match kind {
            StyleKind::Bold => self.bold = enabled,
            StyleKind::Italic => self.italic = enabled,
            StyleKind::Underline => self.underline = enabled,
        }
    }
}

fn has_wordmock_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wordmock"))
}

fn default_font_family() -> String {
    "Roboto".to_owned()
}

fn default_margin() -> f32 {
    76.0
}

fn default_line_spacing() -> f32 {
    1.0
}

fn validate_saved_document(saved: &SavedDocument) -> io::Result<()> {
    if !saved.font_size.is_finite()
        || !(6.0..=96.0).contains(&saved.font_size)
        || !saved.margin.is_finite()
        || !(24.0..=120.0).contains(&saved.margin)
        || !saved.line_spacing.is_finite()
        || !(1.0..=2.5).contains(&saved.line_spacing)
        || !supported_font_family(&saved.font_family)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "document contains invalid font or page settings",
        ));
    }
    let mut previous_end = 0;
    for span in &saved.formats {
        if span.start >= span.end
            || span.end > saved.text.len()
            || !saved.text.is_char_boundary(span.start)
            || !saved.text.is_char_boundary(span.end)
            || span.start < previous_end
            || span.style.is_plain()
            || span
                .style
                .font_size
                .is_some_and(|size| !size.is_finite() || !(6.0..=96.0).contains(&size))
            || span
                .style
                .font_family
                .as_deref()
                .is_some_and(|family| !supported_font_family(family))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "document contains an invalid character-format range",
            ));
        }
        previous_end = span.end;
    }
    Ok(())
}

pub fn supported_font_family(family: &str) -> bool {
    matches!(
        family,
        "System" | "Roboto" | "Open Sans" | "Lora" | "Roboto Mono"
    )
}

fn clamp_range_to_char_boundaries(text: &str, range: Range<usize>) -> Range<usize> {
    let start = range.start.min(text.len());
    let end = range.end.min(text.len());
    let start = if text.is_char_boundary(start) {
        start
    } else {
        (0..start)
            .rev()
            .find(|offset| text.is_char_boundary(*offset))
            .unwrap_or(0)
    };
    let end = if text.is_char_boundary(end) {
        end
    } else {
        (end..=text.len())
            .find(|offset| text.is_char_boundary(*offset))
            .unwrap_or(text.len())
    };
    start.min(end)..end
}

fn changed_byte_ranges(before: &str, after: &str) -> (usize, usize, usize, usize) {
    let before_chars: Vec<char> = before.chars().collect();
    let after_chars: Vec<char> = after.chars().collect();
    let mut prefix = 0;
    while prefix < before_chars.len()
        && prefix < after_chars.len()
        && before_chars[prefix] == after_chars[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < before_chars.len() - prefix
        && suffix < after_chars.len() - prefix
        && before_chars[before_chars.len() - 1 - suffix]
            == after_chars[after_chars.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_start = char_to_byte(before, prefix);
    let new_start = char_to_byte(after, prefix);
    let old_end = char_to_byte(before, before_chars.len() - suffix);
    let new_end = char_to_byte(after, after_chars.len() - suffix);
    (old_start, old_end, new_start, new_end)
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte, _)| byte)
}

fn shift_byte(byte: usize, delta: isize) -> usize {
    byte.checked_add_signed(delta).unwrap_or(0)
}

fn spans_from_runs(runs: Vec<(usize, usize, CharacterStyle)>) -> Vec<FormatSpan> {
    runs.into_iter()
        .filter(|(start, end, style)| start < end && !style.is_plain())
        .map(|(start, end, style)| FormatSpan { start, end, style })
        .collect()
}

fn merge_spans(spans: Vec<FormatSpan>) -> Vec<FormatSpan> {
    let mut merged: Vec<FormatSpan> = Vec::with_capacity(spans.len());
    for span in spans
        .into_iter()
        .filter(|span| span.start < span.end && !span.style.is_plain())
    {
        if let Some(previous) = merged.last_mut()
            && previous.end == span.start
            && previous.style == span.style
        {
            previous.end = span.end;
        } else {
            merged.push(span);
        }
    }
    merged
}
