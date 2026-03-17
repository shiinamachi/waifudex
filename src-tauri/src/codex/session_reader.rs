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

#[cfg(test)]
mod session_reader_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::SessionReader;

    #[test]
    fn reads_only_newly_appended_jsonl_records() {
        let root = create_temp_root();
        let path = root.join("rollout.jsonl");
        write_file(&path, "{\"event\":\"one\"}\n");

        let mut reader = SessionReader::new();
        assert_eq!(
            reader.read_new_lines(&path).expect("initial read succeeds"),
            vec!["{\"event\":\"one\"}".to_string()]
        );
        assert!(reader
            .read_new_lines(&path)
            .expect("unchanged read succeeds")
            .is_empty());

        append_file(&path, "{\"event\":\"two\"}\n");
        assert_eq!(
            reader.read_new_lines(&path).expect("append read succeeds"),
            vec!["{\"event\":\"two\"}".to_string()]
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn buffers_incomplete_trailing_line_until_next_read() {
        let root = create_temp_root();
        let path = root.join("rollout.jsonl");
        write_file(&path, "{\"event\":\"one\"}\n{\"event\":\"two\"");

        let mut reader = SessionReader::new();
        assert_eq!(
            reader.read_new_lines(&path).expect("initial read succeeds"),
            vec!["{\"event\":\"one\"}".to_string()]
        );

        append_file(&path, "}\n");
        assert_eq!(
            reader
                .read_new_lines(&path)
                .expect("follow-up read succeeds"),
            vec!["{\"event\":\"two\"}".to_string()]
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn resets_offset_when_file_shrinks_or_path_changes() {
        let root = create_temp_root();
        let first = root.join("rollout-a.jsonl");
        let second = root.join("rollout-b.jsonl");

        write_file(&first, "{\"event\":\"one\"}\n");
        write_file(&second, "{\"event\":\"other\"}\n");

        let mut reader = SessionReader::new();
        assert_eq!(
            reader
                .read_new_lines(&first)
                .expect("initial read succeeds"),
            vec!["{\"event\":\"one\"}".to_string()]
        );

        write_file(&first, "{}\n");
        assert_eq!(
            reader
                .read_new_lines(&first)
                .expect("shrunken read succeeds"),
            vec!["{}".to_string()]
        );

        assert_eq!(
            reader
                .read_new_lines(&second)
                .expect("path change read succeeds"),
            vec!["{\"event\":\"other\"}".to_string()]
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    fn create_temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("waifudex-reader-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("write temp file");
    }

    fn append_file(path: &Path, contents: &str) {
        use std::io::Write;

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open temp file for append");
        file.write_all(contents.as_bytes())
            .expect("append temp file");
    }
}
