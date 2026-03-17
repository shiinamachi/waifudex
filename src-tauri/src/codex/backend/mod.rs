use std::{io, path::PathBuf, time::SystemTime};

pub mod command_runner;
pub mod local_fs;
pub mod wsl_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCandidate {
    pub session_id: String,
    pub backend_key: String,
    pub path: PathBuf,
    pub modified_at: SystemTime,
    pub size_bytes: u64,
    pub had_recent_append: bool,
}

pub trait SessionBackend: Send {
    fn backend_kind(&self) -> &'static str;
    fn sessions_root_display(&self) -> &str;
    fn sessions_root_available(&self) -> bool;
    fn select_active_session(&mut self) -> io::Result<Option<SessionCandidate>>;
    fn read_new_lines(&mut self, session: &SessionCandidate) -> io::Result<Vec<String>>;
}
