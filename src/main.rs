use clap::{Parser, Subcommand};
use scribe::{
    add_snippet, daemon_status, delete_snippet, display_snippet_manager, run_daemon_worker,
    start_daemon, stop_daemon, update_snippet,
};
use std::env;
use std::process;

#[derive(Parser)]
#[command(
    author = "Gokul <@bahdotsh>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Scribe - A text snippet expansion tool",
    long_about = "Scribe allows you to define text snippets and expand them as you type."
)]
struct CliArgs {
    #[clap(short, long, help = "View and manage your snippets")]
    config: bool,

    #[clap(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new text snippet
    Add {
        #[clap(long, short = 's', help = "Shortcut for the snippet")]
        shortcut: String,

        #[clap(long, short = 'c', help = "The snippet text")]
        snippet: String,
    },
    /// Delete a text snippet by shortcut
    Delete {
        #[clap(long, short, help = "Shortcut of the snippet to delete")]
        shortcut: String,
    },
    /// Update an existing snippet by shortcut
    Update {
        #[clap(long, short = 's', help = "Shortcut of the snippet to update")]
        shortcut: String,

        #[clap(long, short = 'c', help = "New snippet text")]
        snippet: String,
    },
    /// Start the scribe daemon
    Start,
    /// Stop the scribe daemon
    Stop,
    /// Check the status of the scribe daemon
    Status,
}

fn main() {
    // Special hidden flag for daemon worker process
    if env::args().any(|arg| arg == "--daemon-worker") {
        if let Err(e) = run_daemon_worker() {
            eprintln!("Daemon worker failed: {}", e);
            process::exit(1);
        }
        return;
    }

    let args = CliArgs::parse();

    if args.config {
        if let Err(e) = display_snippet_manager() {
            eprintln!("Error displaying snippets: {}", e);
            process::exit(1);
        }
        return;
    }

    // Process subcommands
    let result = match args.commands {
        Some(Commands::Add { shortcut, snippet }) => {
            add_snippet(shortcut, snippet).map(|_| println!("Snippet added successfully"))
        }
        Some(Commands::Delete { shortcut }) => {
            delete_snippet(&shortcut).map(|_| println!("Snippet deleted successfully"))
        }
        Some(Commands::Update { shortcut, snippet }) => {
            update_snippet(&shortcut, snippet).map(|_| println!("Snippet updated successfully"))
        }
        Some(Commands::Start) => start_daemon(),
        Some(Commands::Stop) => stop_daemon(),
        Some(Commands::Status) => daemon_status(),
        None => {
            println!("Use --help for usage information");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
