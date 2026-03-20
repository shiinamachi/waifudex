use std::{
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct SessionReader {
    active_path: Option<PathBuf>,
    offset: u64,
    pending: String,
}

impl SessionReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_new_lines(&mut self, path: &Path) -> io::Result<Vec<String>> {
        if self.active_path.as_deref() != Some(path) {
            self.reset_for(path);
        }

        let metadata = fs::metadata(path)?;
        if metadata.len() < self.offset {
            self.offset = 0;
            self.pending.clear();
        }

        let mut file = fs::File::open(path)?;
        file.seek(SeekFrom::Start(self.offset))?;

        let mut chunk = String::new();
        file.read_to_string(&mut chunk)?;
        self.offset = metadata.len();

        if chunk.is_empty() {
            return Ok(Vec::new());
        }

        let mut combined = std::mem::take(&mut self.pending);
        combined.push_str(&chunk);

        let trailing_newline = combined.ends_with('\n');
        let mut lines: Vec<String> = combined.lines().map(ToString::to_string).collect();

        if !trailing_newline {
            self.pending = lines.pop().unwrap_or_default();
        }

        Ok(lines)
    }

    fn reset_for(&mut self, path: &Path) {
        self.active_path = Some(path.to_path_buf());
        self.offset = 0;
        self.pending.clear();
    }
}
