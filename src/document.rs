use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Document {
    pub text: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub font_size: f32,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: String::new(),
            path: None,
            dirty: false,
            font_size: 12.0,
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

    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        Ok(Self {
            text,
            path: Some(path.to_path_buf()),
            dirty: false,
            font_size: 12.0,
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.as_deref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "document has no file path")
        })?;
        fs::write(path, &self.text)?;
        self.dirty = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        fs::write(path, &self.text)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
