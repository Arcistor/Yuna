use std::path::Path;

use digital_ghost::installer::{
    render_launchd_plist, render_systemd_unit, LaunchdConfig, SystemdConfig,
};

#[test]
fn launchd_plist_contains_binary_paths_and_home_environment() {
    let plist = render_launchd_plist(&LaunchdConfig {
        label: "com.example.digital-ghost".to_string(),
        ghost_binary: Path::new("/Users/me/.local/bin/ghost").to_path_buf(),
        working_directory: Path::new("/Users/me").to_path_buf(),
        home: Path::new("/Users/me").to_path_buf(),
        stdout_log: Path::new("/Users/me/.ghost/ghost.out.log").to_path_buf(),
        stderr_log: Path::new("/Users/me/.ghost/ghost.err.log").to_path_buf(),
    });

    assert!(plist.contains("<key>Label</key>"));
    assert!(plist.contains("com.example.digital-ghost"));
    assert!(plist.contains("/Users/me/.local/bin/ghost"));
    assert!(plist.contains("<key>RunAtLoad</key>"));
    assert!(plist.contains("<key>KeepAlive</key>"));
    assert!(plist.contains("<key>HOME</key>"));
    assert!(plist.contains("/Users/me/.ghost/ghost.err.log"));
}

#[test]
fn systemd_unit_contains_execstart_and_restart_policy() {
    let unit = render_systemd_unit(&SystemdConfig {
        description: "Digital Ghost".to_string(),
        ghost_binary: Path::new("/home/me/.local/bin/ghost").to_path_buf(),
        working_directory: Path::new("/home/me").to_path_buf(),
        home: Path::new("/home/me").to_path_buf(),
    });

    assert!(unit.contains("[Unit]"));
    assert!(unit.contains("Description=Digital Ghost"));
    assert!(unit.contains("ExecStart=/home/me/.local/bin/ghost"));
    assert!(unit.contains("WorkingDirectory=/home/me"));
    assert!(unit.contains("Environment=HOME=/home/me"));
    assert!(unit.contains("Restart=on-failure"));
    assert!(unit.contains("WantedBy=default.target"));
}

#[test]
fn launchd_plist_escapes_xml_sensitive_characters() {
    let plist = render_launchd_plist(&LaunchdConfig {
        label: "com.example.digital-ghost".to_string(),
        ghost_binary: Path::new("/Users/me/A&B/ghost").to_path_buf(),
        working_directory: Path::new("/Users/me/<work>").to_path_buf(),
        home: Path::new("/Users/me").to_path_buf(),
        stdout_log: Path::new("/Users/me/.ghost/out.log").to_path_buf(),
        stderr_log: Path::new("/Users/me/.ghost/err.log").to_path_buf(),
    });

    assert!(plist.contains("A&amp;B"));
    assert!(plist.contains("&lt;work&gt;"));
}
