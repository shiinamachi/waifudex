use super::StatusKind;

pub fn parse_status_line(line: &str) -> Option<StatusKind> {
    let normalized = line.to_ascii_lowercase();

    if normalized.contains("thinking") {
        return Some(StatusKind::Thinking);
    }
    if normalized.contains("writing") || normalized.contains("edit") {
        return Some(StatusKind::Writing);
    }
    if normalized.contains("test") {
        return Some(StatusKind::RunningTests);
    }
    if normalized.contains("success") || normalized.contains("done") {
        return Some(StatusKind::Success);
    }
    if normalized.contains("error") || normalized.contains("fail") {
        return Some(StatusKind::Error);
    }

    None
}
