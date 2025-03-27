use clap::{Parser, Subcommand};
use crossterm::execute;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::LeaveAlternateScreen;
use scribe::display_scribe_dashboard;
use scribe::interactive_add;
use scribe::AddResult;
use scribe::{
    add_snippet, daemon_status, delete_snippet, display_snippet_manager, is_daemon_running,
    run_daemon_worker, start_daemon, stop_daemon, update_snippet,
};
use std::env;
use std::io::stdout;
use std::process;

#[derive(Parser)]
#[command(
    author = "Gokul <@bahdotsh>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Scribe - A text snippet expansion tool",
    long_about = "Scribe allows you to define text snippets and expand them as you type."
)]
struct CliArgs {
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
    /// Add a new snippet interactively
    New,
    /// Start the scribe daemon
    Start,
    /// Stop the scribe daemon
    Stop,
    /// Check the status of the scribe daemon
    Status,
    /// List all the configs
    List,
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
        Some(Commands::New) => {
            // First, disable raw mode to ensure clean state
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

            // Now add the snippet interactively
            let interactive_result = interactive_add();

            // Reset terminal state again
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

            match interactive_result {
                AddResult::Added => {
                    // Snippet was added successfully
                    println!("Snippet added successfully!");
                    println!("Press Enter to view your snippets...");

                    // Wait for user to press Enter
                    let mut input = String::new();
                    let _ = std::io::stdin().read_line(&mut input);

                    // Launch snippet manager as a new process - handle errors without ?
                    match std::env::current_exe() {
                        Ok(exe) => match std::process::Command::new(exe).arg("list").status() {
                            Ok(status) => {
                                if !status.success() {
                                    eprintln!("Snippet manager exited with error code: {}", status);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to execute snippet manager: {}", e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to get current executable path: {}", e);
                        }
                    }

                    Ok(())
                }
                AddResult::Cancelled => {
                    // User canceled
                    println!("Snippet addition canceled.");
                    Ok(())
                }
                AddResult::Error(e) => Err(e),
            }
        }
        Some(Commands::List) => display_snippet_manager(),
        None => {
            // When no command is provided, launch the main UI
            display_main_ui()
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn display_main_ui() -> scribe::Result<()> {
    // First check if daemon is running
    let daemon_status = is_daemon_running()?;

    // Now launch the UI with the daemon status information
    display_scribe_dashboard(daemon_status)
}
