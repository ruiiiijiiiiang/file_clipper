use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::models::Action;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "cp")]
    #[command(alias = "c")]
    #[command(alias = "y")]
    Copy {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    #[command(alias = "mv")]
    #[command(alias = "d")]
    #[command(alias = "x")]
    Cut {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    #[command(alias = "ln")]
    #[command(alias = "s")]
    Link {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    #[command(alias = "p")]
    #[command(alias = "v")]
    Paste {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    #[command(alias = "l")]
    #[command(alias = "ls")]
    List,

    #[command(alias = "h")]
    History,
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
    }
}
