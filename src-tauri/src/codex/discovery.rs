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
