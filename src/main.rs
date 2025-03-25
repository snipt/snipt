use clap::Parser;
use scribe::{Commands, Scribe};
use scribe::{
    add_snippet, daemon_status, delete_snippet, print_scribe, start_daemon, stop_daemon,
    update_snippet,
};
use std::env;
use std::process;

fn main() {
    let scribe = Scribe::parse();

    // Special hidden flag for daemon worker process
    if env::args().any(|arg| arg == "--daemon-worker") {
        if let Err(e) = scribe::run_daemon_worker() {
            eprintln!("Daemon worker failed: {}", e);
            process::exit(1);
        }
        return;
    }

    if scribe.config {
        print_scribe().unwrap();
    }
    match scribe.commands {
        Some(Commands::Add { shortcut, snippet }) => add_snippet(shortcut, snippet),
        Some(Commands::Delete { shortcut }) => delete_snippet(shortcut),
        Some(Commands::Update { shortcut, snippet }) => update_snippet(shortcut, snippet),
        Some(Commands::Start) => {
            if let Err(e) = start_daemon() {
                eprintln!("Failed to start daemon: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::Stop) => {
            if let Err(e) = stop_daemon() {
                eprintln!("Failed to stop daemon: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::Status) => {
            if let Err(e) = daemon_status() {
                eprintln!("Failed to check daemon status: {}", e);
                process::exit(1);
            }
        }
        None => return,
    }
}
