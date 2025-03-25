use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author = "Gokul <@bahdotsh>",
    version = env!("CARGO_PKG_VERSION"),
    about = "scribe",
)]
pub struct Scribe {
    #[clap(long, short, help = "Start the scribe daemon")]
    pub daemon: bool,

    #[clap(long = "stop-daemon", short = 'k', help = "Stop the scribe daemon")]
    pub stop_daemon: bool,

    #[clap(
        long = "daemon-status",
        short = 't',
        help = "Check if the daemon is running"
    )]
    pub daemon_status: bool,

    #[clap(short, long, help = "View the contents of the config file")]
    pub config: bool,

    #[clap(subcommand)]
    pub commands: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a text snippet
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
}
