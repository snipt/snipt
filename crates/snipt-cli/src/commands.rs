use crate::cli::Commands;
use crate::utils::display_main_ui;
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use snipt_core::{add_snippet, delete_snippet, update_snippet, Result};
use snipt_daemon::{daemon_status, daemon_worker_entry, start_daemon, stop_daemon};
use snipt_server::server::http_server::{check_api_server_health, diagnose_api_server};
use snipt_server::server::start_api_server;
use snipt_server::server::utils::get_api_server_port;
use snipt_ui::{display_snippet_manager, interactive_add, AddResult};
use std::io::stdout;
use std::thread;
use std::time::Duration;

pub fn handle_command(command: Option<Commands>) -> Result<()> {
    match command {
        Some(command) => handle_subcommand(command),
        None => display_main_ui(), // Default: show main UI when no command provided
    }
}

fn handle_subcommand(command: Commands) -> Result<()> {
    match command {
        Commands::Add { shortcut, snippet } => {
            add_snippet(shortcut, snippet).map(|_| println!("Snippet added successfully"))
        }
        Commands::Delete { shortcut } => {
            delete_snippet(&shortcut).map(|_| println!("Snippet deleted successfully"))
        }
        Commands::Update { shortcut, snippet } => {
            update_snippet(&shortcut, snippet).map(|_| println!("Snippet updated successfully"))
        }
        Commands::Start { port } => start_daemon(port),
        Commands::Stop => stop_daemon(),
        Commands::Status => daemon_status(),
        Commands::New => handle_interactive_add(),
        Commands::List => display_snippet_manager(),
        Commands::Serve { port } => handle_serve_command(port),
        Commands::Port => handle_port_command(),
        Commands::ApiStatus => check_api_server_health(),
        Commands::ApiDiagnose => diagnose_api_server(),
        Commands::DaemonWorker => daemon_worker_entry(),
    }
}

fn handle_interactive_add() -> Result<()> {
    // First, fully reset terminal state
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);

    // Clear the screen explicitly to prevent artifacts
    println!("\x1B[2J\x1B[1;1H");

    // Add the snippet interactively
    let interactive_result = interactive_add();

    // Reset terminal state again
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);

    // Clear the screen again
    println!("\x1B[2J\x1B[1;1H");

    match interactive_result {
        AddResult::Added => {
            thread::sleep(Duration::from_millis(300));
            // Launch the snippet manager UI
            if let Err(e) = display_snippet_manager() {
                eprintln!("Error displaying snippets: {}", e);
            }
        }
        AddResult::Cancelled => {
            println!("Operation canceled.");
            thread::sleep(Duration::from_millis(300));
        }
        AddResult::Error(e) => {
            // Print the error but don't return it from main
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(500));
        }
    }

    // Regardless of what happened above, always return to main menu
    // Start with a completely fresh terminal state
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
    println!("\x1B[2J\x1B[1;1H");

    display_main_ui()
}

fn handle_serve_command(port: u16) -> Result<()> {
    // Start API server only in a properly configured runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Block the current thread with the server
    runtime.block_on(async {
        println!("Starting standalone API server on port {}...", port);
        start_api_server(port).await
    })
}

fn handle_port_command() -> Result<()> {
    match get_api_server_port() {
        Ok(port) => {
            println!("snipt API server is running on port {}", port);
            println!("UI available at: http://localhost:{}", port);
            Ok(())
        }
        Err(_) => {
            println!("snipt API server port information not found.");
            println!(
                "The API server may not be running or was started without saving port information."
            );
            println!("Try 'snipt status' for more details.");
            Ok(())
        }
    }
}
