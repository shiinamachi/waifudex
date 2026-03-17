use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCandidate {
    pub path: PathBuf,
    pub modified_at: SystemTime,
    pub size_bytes: u64,
    pub had_recent_append: bool,
}

#[derive(Debug, Default)]
pub struct SessionDiscovery {
    root: PathBuf,
    previous_sizes: HashMap<PathBuf, u64>,
}

impl SessionDiscovery {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            previous_sizes: HashMap::new(),
        }
    }

    pub fn select_active_session(&mut self) -> io::Result<Option<SessionCandidate>> {
        let mut candidates = self.scan_rollouts()?;
        candidates.sort_by(|left, right| {
            right
                .had_recent_append
                .cmp(&left.had_recent_append)
                .then_with(|| right.modified_at.cmp(&left.modified_at))
                .then_with(|| right.path.cmp(&left.path))
        });

        Ok(candidates.into_iter().next())
    }

    fn scan_rollouts(&mut self) -> io::Result<Vec<SessionCandidate>> {
        let mut candidates = Vec::new();
        let mut seen_paths = Vec::new();

        visit_rollouts(&self.root, &mut |path| {
            let metadata = fs::metadata(path)?;
            let modified_at = metadata.modified()?;
            let size_bytes = metadata.len();
            let previous_size = self.previous_sizes.get(path).copied().unwrap_or(size_bytes);
            let had_recent_append = size_bytes > previous_size;

            self.previous_sizes.insert(path.to_path_buf(), size_bytes);
            seen_paths.push(path.to_path_buf());
            candidates.push(SessionCandidate {
                path: path.to_path_buf(),
                modified_at,
                size_bytes,
                had_recent_append,
            });

            Ok(())
        })?;

        self.previous_sizes
            .retain(|path, _| seen_paths.iter().any(|seen| seen == path));

        Ok(candidates)
    }
}

fn visit_rollouts(root: &Path, visitor: &mut dyn FnMut(&Path) -> io::Result<()>) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error),
        };
        let path = entry.path();

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error),
        };

        if file_type.is_dir() {
            visit_rollouts(&path, visitor)?;
            continue;
        }

        if is_rollout_path(&path) {
            match visitor(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
                Err(error) => return Err(error),
            }
        }
    }

    Ok(())
}

fn is_rollout_path(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(name) if name.starts_with("rollout-") && name.ends_with(".jsonl")
    )
}

#[cfg(test)]
mod discovery_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::SessionDiscovery;

    #[test]
    fn finds_the_newest_rollout_file() {
        let root = create_temp_sessions_root();
        let older = root.join("2026/03/17/rollout-older.jsonl");
        write_rollout(&older, "{}\n");
        thread::sleep(Duration::from_millis(20));

        let newer = root.join("2026/03/17/rollout-newer.jsonl");
        write_rollout(&newer, "{}\n");

        let mut discovery = SessionDiscovery::new(root.clone());
        let active = discovery
            .select_active_session()
            .expect("discovery succeeds")
            .expect("active session exists");

        assert_eq!(active.path, newer);

        fs::remove_dir_all(root).expect("cleanup temp sessions root");
    }

    #[test]
    fn prefers_recent_append_activity_over_an_older_file() {
        let root = create_temp_sessions_root();
        let older = root.join("2026/03/17/rollout-older.jsonl");
        let newer = root.join("2026/03/17/rollout-newer.jsonl");

        write_rollout(&older, "{}\n");
        thread::sleep(Duration::from_millis(20));
        write_rollout(&newer, "{}\n");

        let mut discovery = SessionDiscovery::new(root.clone());
        let initial = discovery
            .select_active_session()
            .expect("initial discovery succeeds")
            .expect("initial active session exists");
        assert_eq!(initial.path, newer);

        thread::sleep(Duration::from_millis(20));
        append_rollout(&older, "{\"event\":\"message_delta\"}\n");

        let active = discovery
            .select_active_session()
            .expect("updated discovery succeeds")
            .expect("updated active session exists");

        assert_eq!(active.path, older);
        assert!(active.had_recent_append);

        fs::remove_dir_all(root).expect("cleanup temp sessions root");
    }

    #[test]
    fn returns_no_active_session_when_no_rollout_files_exist() {
        let root = create_temp_sessions_root();
        let mut discovery = SessionDiscovery::new(root.clone());

        let active = discovery
            .select_active_session()
            .expect("discovery succeeds without rollout files");

        assert!(active.is_none());

        fs::remove_dir_all(root).expect("cleanup temp sessions root");
    }

    #[cfg(unix)]
    #[test]
    fn ignores_unreadable_descendant_directories_during_discovery() {
        use std::os::unix::fs::PermissionsExt;

        let root = create_temp_sessions_root();
        let readable = root.join("2026/03/17/rollout-readable.jsonl");
        let blocked_dir = root.join("2026/03/18/blocked");

        write_rollout(&readable, "{}\n");
        fs::create_dir_all(&blocked_dir).expect("create blocked directory");
        fs::set_permissions(&blocked_dir, fs::Permissions::from_mode(0o000))
            .expect("mark blocked directory unreadable");

        let mut discovery = SessionDiscovery::new(root.clone());
        let active = discovery
            .select_active_session()
            .expect("discovery should skip unreadable descendants")
            .expect("readable rollout should still be selected");

        assert_eq!(active.path, readable);

        fs::set_permissions(&blocked_dir, fs::Permissions::from_mode(0o755))
            .expect("restore blocked directory permissions");
        fs::remove_dir_all(root).expect("cleanup temp sessions root");
    }

    fn create_temp_sessions_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("waifudex-discovery-{unique}"));
        fs::create_dir_all(&root).expect("create temp sessions root");
        root
    }

    fn write_rollout(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create rollout parent directories");
        }
        fs::write(path, contents).expect("write rollout file");
    }

    fn append_rollout(path: &Path, contents: &str) {
        use std::io::Write;

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open rollout file for append");
        file.write_all(contents.as_bytes())
            .expect("append rollout contents");
    }
}
