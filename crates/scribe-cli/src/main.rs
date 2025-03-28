mod cli;
mod commands;
mod utils;

use clap::Parser;
use cli::Scribe;
use commands::handle_command;
use std::env;
use std::process;

fn main() {
    // Special hidden flag for daemon worker process
    if env::args().any(|arg| arg == "--daemon-worker") {
        if let Err(e) = scribe_daemon::run_daemon_worker() {
            eprintln!("Daemon worker failed: {}", e);
            process::exit(1);
        }
        return;
    }

    let args = Scribe::parse();
    let result = handle_command(args.commands);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
