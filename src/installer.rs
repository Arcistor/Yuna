use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchdConfig {
    pub label: String,
    pub yuna_binary: PathBuf,
    pub working_directory: PathBuf,
    pub home: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemdConfig {
    pub description: String,
    pub yuna_binary: PathBuf,
    pub working_directory: PathBuf,
    pub home: PathBuf,
}

pub fn render_launchd_plist(config: &LaunchdConfig) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{yuna_binary}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>{working_directory}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>HOME</key>
    <string>{home}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{stdout_log}</string>
  <key>StandardErrorPath</key>
  <string>{stderr_log}</string>
</dict>
</plist>
"#,
        label = escape_xml(&config.label),
        yuna_binary = escape_xml(&config.yuna_binary.display().to_string()),
        working_directory = escape_xml(&config.working_directory.display().to_string()),
        home = escape_xml(&config.home.display().to_string()),
        stdout_log = escape_xml(&config.stdout_log.display().to_string()),
        stderr_log = escape_xml(&config.stderr_log.display().to_string())
    )
}

pub fn render_systemd_unit(config: &SystemdConfig) -> String {
    format!(
        r#"[Unit]
Description={description}
After=default.target

[Service]
Type=simple
ExecStart={yuna_binary}
WorkingDirectory={working_directory}
Environment=HOME={home}
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
"#,
        description = escape_systemd(&config.description),
        yuna_binary = escape_systemd(&config.yuna_binary.display().to_string()),
        working_directory = escape_systemd(&config.working_directory.display().to_string()),
        home = escape_systemd(&config.home.display().to_string())
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_systemd(value: &str) -> String {
    value.replace('\n', " ")
}
