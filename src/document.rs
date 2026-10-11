use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const MAX_UNDO_STEPS: usize = 100;
const TYPE_GROUP_DELAY: Duration = Duration::from_millis(700);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl CharacterStyle {
    pub fn is_plain(self) -> bool {
        !self.bold && !self.italic && !self.underline
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatSpan {
    /// UTF-8 byte offsets into the document text.
    pub start: usize,
    pub end: usize,
    pub style: CharacterStyle,
}

#[derive(Debug)]
pub struct Document {
    pub text: String,
    pub formats: Vec<FormatSpan>,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub font_size: f32,
    undo_stack: Vec<DocumentSnapshot>,
    redo_stack: Vec<DocumentSnapshot>,
    last_text_edit: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
struct DocumentSnapshot {
    text: String,
    formats: Vec<FormatSpan>,
    font_size: f32,
}

#[derive(Serialize, Deserialize)]
struct SavedDocument {
    format_version: u32,
    text: String,
    formats: Vec<FormatSpan>,
    font_size: f32,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: String::new(),
            formats: Vec::new(),
            path: None,
            dirty: false,
            font_size: 12.0,
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
            .map_or(CharacterStyle::default(), |span| span.style)
    }

    pub fn toggle_style(&mut self, bytes: Range<usize>, kind: StyleKind) {
        let range = clamp_range_to_char_boundaries(&self.text, bytes);
        if range.is_empty() {
            return;
        }

        let disable = self.has_style(range.clone(), kind);
        self.push_undo_snapshot();
        let mut runs = self.style_runs();
        for (start, end, style) in &mut runs {
            if *end <= range.start || *start >= range.end {
                continue;
            }
            if disable {
                style.set(kind, false);
            } else {
                style.set(kind, true);
            }
        }
        self.formats = spans_from_runs(runs);
        self.mark_changed();
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
                        style: span.style,
                    });
                }
            }
            if span.end > old_end {
                let start = span.start.max(old_end);
                if start < span.end {
                    adjusted.push(FormatSpan {
                        start: shift_byte(start, delta),
                        end: shift_byte(span.end, delta),
                        style: span.style,
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

    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let (text, formats, font_size) = if has_wordmock_extension(path) {
            let saved: SavedDocument = serde_json::from_slice(&fs::read(path)?)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if saved.format_version != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "unsupported WordMock document version {}",
                        saved.format_version
                    ),
                ));
            }
            validate_saved_document(&saved)?;
            (saved.text, saved.formats, saved.font_size)
        } else {
            (fs::read_to_string(path)?, Vec::new(), 12.0)
        };

        Ok(Self {
            text,
            formats,
            path: Some(path.to_path_buf()),
            dirty: false,
            font_size,
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
                format_version: 1,
                text: self.text.clone(),
                formats: self.formats.clone(),
                font_size: self.font_size,
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
            runs.push((span.start, span.end, span.style));
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
    }

    fn mark_changed(&mut self) {
        self.dirty = true;
        self.redo_stack.clear();
        self.last_text_edit = None;
    }
}

impl CharacterStyle {
    fn style(self, kind: StyleKind) -> bool {
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

fn validate_saved_document(saved: &SavedDocument) -> io::Result<()> {
    if !saved.font_size.is_finite() || !(6.0..=96.0).contains(&saved.font_size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "document font size must be between 6 and 96 points",
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
