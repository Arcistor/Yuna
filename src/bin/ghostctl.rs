use anyhow::Result;
use clap::{Parser, Subcommand};
use digital_ghost::app::open_default_store;

#[derive(Debug, Parser)]
#[command(name = "ghostctl", about = "Inspect the Digital Ghost daemon state")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Notes,
    Mood,
    Silence { duration: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = open_default_store()?;

    match cli.command {
        Command::Status => {
            let mood = store.get_mood()?;
            let last_event = store
                .last_event_time()?
                .map(|time| time.to_string())
                .unwrap_or_else(|| "never".to_string());
            let note_count = store.list_undeleted_notes()?.len();
            println!("running: unknown");
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
