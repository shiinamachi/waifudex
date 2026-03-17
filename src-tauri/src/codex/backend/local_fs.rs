use std::{
    io,
    path::{Path, PathBuf},
};

use crate::codex::{discovery::SessionDiscovery, session_reader::SessionReader};

use super::{SessionBackend, SessionCandidate};

pub struct LocalFsBackend {
    sessions_root: PathBuf,
    sessions_root_display: String,
    discovery: SessionDiscovery,
    reader: SessionReader,
}

impl LocalFsBackend {
    pub fn new(sessions_root: PathBuf) -> Self {
        let sessions_root_display = sessions_root.display().to_string();
        let discovery_root = sessions_root.clone();

        Self {
            sessions_root,
            sessions_root_display,
            discovery: SessionDiscovery::new(discovery_root),
            reader: SessionReader::new(),
        }
    }
}

impl SessionBackend for LocalFsBackend {
    fn backend_kind(&self) -> &'static str {
        "local_fs"
    }

    fn sessions_root_display(&self) -> &str {
        &self.sessions_root_display
    }

    fn sessions_root_available(&self) -> bool {
        self.sessions_root.is_dir() && std::fs::read_dir(&self.sessions_root).is_ok()
    }

    fn select_active_session(&mut self) -> io::Result<Option<SessionCandidate>> {
        let candidate = self.discovery.select_active_session()?;
        Ok(candidate.map(|candidate| {
            let path = candidate.path;
            let session_id = path.display().to_string();
            SessionCandidate {
                session_id: session_id.clone(),
                backend_key: session_id,
                path,
                modified_at: candidate.modified_at,
                size_bytes: candidate.size_bytes,
                had_recent_append: candidate.had_recent_append,
            }
        }))
    }

    fn read_new_lines(&mut self, session: &SessionCandidate) -> io::Result<Vec<String>> {
        self.reader.read_new_lines(Path::new(&session.backend_key))
    }
}
