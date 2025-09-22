use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::models::Action;

#[derive(Parser)]
#[command(author, version, about, long_about = None, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files to the clipboard
    #[command(alias = "cp")]
    #[command(alias = "c")]
    #[command(alias = "y")]
    Copy {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    /// Cut files to the clipboard
    #[command(alias = "mv")]
    #[command(alias = "d")]
    #[command(alias = "x")]
    Cut {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    /// Create symbolic links to files and add them to the clipboard
    #[command(alias = "ln")]
    #[command(alias = "s")]
    Link {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    /// Paste files from the clipboard to the specified directory
    #[command(alias = "p")]
    #[command(alias = "v")]
    Paste {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// List files currently in the clipboard
    #[command(alias = "l")]
    #[command(alias = "ls")]
    List,

    /// Show the history of clipboard operations
    #[command(alias = "h")]
    History,

    /// Clear the clipboard and history
    Clear,
}

pub fn handle_cli() -> Action {
    let cli = Cli::parse();

    match cli.command {
        Commands::Copy { paths } => Action::Copy(paths),
        Commands::Cut { paths } => Action::Cut(paths),
        Commands::Link { paths } => Action::Link(paths),
        Commands::Paste { path } => Action::Paste(path),
        Commands::List => Action::Clipboard,
        Commands::History => Action::History,
        Commands::Clear => Action::Clear,
    }
}
