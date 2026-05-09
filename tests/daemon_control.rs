use std::fs;
use std::path::Path;

use digital_ghost::app::{
    daemon_status_from_pid_file, read_pid_file, remove_pid_file, write_pid_file, DaemonState,
};
use tempfile::tempdir;

#[test]
fn pid_file_round_trips_process_id() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ghost.pid");

    write_pid_file(&path, 42).unwrap();

    assert_eq!(read_pid_file(&path).unwrap(), Some(42));
    remove_pid_file(&path).unwrap();
    assert_eq!(read_pid_file(&path).unwrap(), None);
}

#[test]
fn daemon_status_reports_stopped_when_pid_file_is_missing() {
    let dir = tempdir().unwrap();
    let status = daemon_status_from_pid_file(&dir.path().join("ghost.pid")).unwrap();

    assert_eq!(status.state, DaemonState::Stopped);
    assert_eq!(status.pid, None);
}

#[test]
fn daemon_status_removes_stale_pid_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ghost.pid");
    write_pid_file(&path, 999_999_999).unwrap();

    let status = daemon_status_from_pid_file(&path).unwrap();

    assert_eq!(status.state, DaemonState::Stale);
    assert_eq!(status.pid, Some(999_999_999));
    assert!(!path.exists());
}

#[test]
fn daemon_status_reports_current_process_as_running() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ghost.pid");
    let current_pid = std::process::id();
    write_pid_file(&path, current_pid).unwrap();

    let status = daemon_status_from_pid_file(&path).unwrap();

    assert_eq!(status.state, DaemonState::Running);
    assert_eq!(status.pid, Some(current_pid));
}

#[test]
fn malformed_pid_file_is_stale_and_removed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ghost.pid");
    fs::write(&path, "not-a-pid").unwrap();

    let status = daemon_status_from_pid_file(Path::new(&path)).unwrap();

    assert_eq!(status.state, DaemonState::Stale);
    assert_eq!(status.pid, None);
    assert!(!path.exists());
}
