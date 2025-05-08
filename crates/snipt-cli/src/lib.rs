pub mod cli;
pub mod commands;
pub mod utils;

use clap::Parser;
use cli::Snipt;
use commands::handle_command;
use std::env;
use std::process;

/// Run the snipt CLI application
pub fn run_main() {
    // Special hidden flag for daemon worker process
    if env::args().any(|arg| arg == "--daemon-worker") {
        if let Err(e) = snipt_daemon::run_daemon_worker() {
            eprintln!("Daemon worker failed: {}", e);
            process::exit(1);
        }
        return;
    }

    let args = Snipt::parse();
    let result = handle_command(args.commands);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
