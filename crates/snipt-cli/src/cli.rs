use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[command(
    author = "Gokul <@bahdotsh>",
    version = env!("CARGO_PKG_VERSION"),
    about = "snipt - A text snippet expansion tool",
    long_about = "snipt allows you to define text snippets and expand them as you type."
)]
pub struct Snipt {
    #[clap(subcommand)]
    pub commands: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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
    /// Start the daemon and API server for UI
    Start {
        #[clap(long, short, default_value = "3000", help = "Port for the API server")]
        port: u16,
    },
    /// Stop the snipt daemon
    Stop,
    /// Check the status of the snipt daemon
    Status,
    /// List all the configs
    List,
    /// Start just the API server (without daemon) for the Electron UI
    Serve {
        #[clap(long, short, default_value = "3000", help = "Port to listen on")]
        port: u16,
    },
    /// Show the API server port
    Port,
    /// Check if the API server is responsive
    ApiStatus,
    /// Diagnose API server issues
    ApiDiagnose,
    // Hidden command used internally to run the daemon worker
    #[clap(hide = true)]
    DaemonWorker,
}
