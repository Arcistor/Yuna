use anyhow::Result;
use clap::{Parser, Subcommand};
use digital_ghost::app::{
    daemon_status, open_default_store, start_daemon_process, stop_daemon_process, DaemonState,
};

#[derive(Debug, Parser)]
#[command(name = "ghostctl", about = "Inspect the Digital Ghost daemon state")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start,
    Stop,
    Status,
    Notes,
    Mood,
    Silence { duration: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = open_default_store()?;

    match cli.command {
        Command::Start => {
            let pid = start_daemon_process()?;
            println!("started: {pid}");
        }
        Command::Stop => match stop_daemon_process()? {
            Some(pid) => println!("stopped: {pid}"),
            None => println!("stopped: already"),
        },
        Command::Status => {
            let daemon = daemon_status()?;
            let mood = store.get_mood()?;
            let last_event = store
                .last_event_time()?
                .map(|time| time.to_string())
                .unwrap_or_else(|| "never".to_string());
            let note_count = store.list_undeleted_notes()?.len();
            println!("running: {}", matches!(daemon.state, DaemonState::Running));
            if let Some(pid) = daemon.pid {
                println!("pid: {pid}");
            }
            if daemon.state == DaemonState::Stale {
                println!("pid_status: stale_removed");
            }
            println!("mood: {mood}");
            println!("last_event: {last_event}");
            println!("unread_notes: {note_count}");
        }
        Command::Notes => {
            for note in store.list_undeleted_notes()? {
                if note.read_at.is_none() {
                    println!("{}", note.path.display());
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
    }

    Ok(())
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
