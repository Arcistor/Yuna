use anyhow::Result;
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use yuna::app::{
    daemon_status, open_default_store, start_daemon_process, stop_daemon_process, DaemonState,
};
use yuna::config::Config;

#[derive(Debug, Parser)]
#[command(name = "yunactl", about = "Inspect the Yuna daemon state")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the Yuna daemon
    Start,
    /// Stop the Yuna daemon
    Stop {
        /// Also stop the Ollama service
        #[arg(short, long)]
        all: bool,
    },
    /// Show current status, mood, and unread note count
    Status,
    /// List notes dropped by Yuna
    Notes {
        /// Show all notes including those already read
        #[arg(short, long)]
        all: bool,
        /// Show only notes that have been read
        #[arg(short, long)]
        read: bool,
        /// Delete all notes (files and database records)
        #[arg(short, long)]
        clear: bool,
    },
    /// Show Yuna's current emotional state
    Mood,
    /// Silence Yuna for a specific duration (e.g., 2h, 30m, 1d)
    Silence {
        /// Duration of silence (e.g., 1h, 30m)
        duration: String,
    },
    /// Show recent filesystem events recorded by Yuna
    Log {
        /// Number of recent events to show
        #[arg(short, long, default_value_t = 30)]
        lines: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = open_default_store()?;

    match cli.command {
        Command::Start => {
            let pid = start_daemon_process()?;
            println!("started: {pid}");
        }
        Command::Stop { all } => {
            match stop_daemon_process()? {
                Some(pid) => println!("stopped yuna: {pid}"),
                None => println!("stopped yuna: already"),
            }
            if all {
                yuna::app::stop_ollama_process()?;
                println!("stopped ollama");
            }
        }
        Command::Status => {
            let config = Config::load()?;
            let daemon = daemon_status()?;
            let mood = store.get_mood()?;
            let last_event = store
                .last_event_time()?
                .map(|time| time.to_string())
                .unwrap_or_else(|| "never".to_string());
            let note_count = store
                .list_undeleted_notes()?
                .into_iter()
                .filter(|n| n.read_at.is_none())
                .count();
            let ollama_online = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(yuna::app::ollama_is_running(&config));

            println!("running: {}", matches!(daemon.state, DaemonState::Running));
            if let Some(pid) = daemon.pid {
                println!("pid: {pid}");
            }
            if daemon.state == DaemonState::Stale {
                println!("pid_status: stale_removed");
            }
            println!("mood: {mood}");
            println!("language: {}", config.yuna.language);
            println!("ollama: {}", if ollama_online { "online" } else { "offline" });
            println!("last_event: {last_event}");
            println!("unread_notes: {note_count}");
        }
        Command::Notes { all, read, clear } => {
            if clear {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(yuna::haunter::clear_all_notes(&store))?;
                println!("all notes cleared");
            } else {
                for note in store.list_undeleted_notes()? {
                    if all {
                        let status = if note.read_at.is_some() {
                            "[READ]"
                        } else {
                            "[NEW]"
                        };
                        println!("{:<6} {}", status, note.path.display());
                    } else if read {
                        if note.read_at.is_some() {
                            println!("{}", note.path.display());
                        }
                    } else {
                        // Default: unread only
                        if note.read_at.is_none() {
                            println!("{}", note.path.display());
                        }
                    }
                }
            }
        }
        Command::Mood => {
            println!("{}", store.get_mood()?);
        }
        Command::Silence { duration } => {
            let seconds = parse_duration_seconds(&duration)?;
            let until = chrono::Utc::now().timestamp() + seconds;
            store.set_silenced_until(until)?;
            println!("silenced_until: {until}");
        }
        Command::Log { lines } => {
            let events = store.recent_events(lines)?;
            if events.is_empty() {
                println!("no events recorded");
                return Ok(());
            }
            for event in events {
                let time = Local
                    .timestamp_opt(event.timestamp, 0)
                    .single()
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| event.timestamp.to_string());
                let label = format_event_label(event.kind.as_str(), &event.path);
                let path = event.path.display();
                println!("[{time}] {label:<18} {path}");
            }
        }
    }

    Ok(())
}

fn format_event_label(kind: &str, path: &std::path::Path) -> String {
    match kind {
        "rename" if !path.exists() => "RENAME (trashed)".to_string(),
        "rename" => "RENAME".to_string(),
        "delete" => "DELETE".to_string(),
        "create" => "CREATE".to_string(),
        "modify" => "MODIFY".to_string(),
        other => other.to_uppercase(),
    }
}

fn parse_duration_seconds(value: &str) -> Result<i64> {
    let (number, unit) = value.split_at(value.len().saturating_sub(1));
    let amount: i64 = number.parse()?;
    let multiplier = match unit {
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => 60 * 60,
    };
    Ok(amount * multiplier)
}
