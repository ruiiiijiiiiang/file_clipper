use std::{error::Error, process};

mod cli;
mod file_handler;
mod models;
mod record_handler;
mod tui;

use crate::cli::handle_cli;
use crate::file_handler::{handle_copy, handle_cut, handle_paste};
use crate::models::{Action, TuiMode};
use crate::tui::enter_tui_mode;

fn main() {
    let result: Result<(), Box<dyn Error>> = match handle_cli() {
        Ok(action) => match action {
            Action::Copy(paths) => handle_copy(paths),
            Action::Cut(paths) => handle_cut(paths),
            Action::Paste(path) => handle_paste(path),
            Action::Clipboard => enter_tui_mode(TuiMode::Clipboard),
            Action::History => enter_tui_mode(TuiMode::History),
            Action::Help => {
                eprintln!("Commands: copy <path>, cut <path>, paste, list, history");
                Ok(())
            }
        },
        Err(error) => {
            eprintln!("{:?}", error);
            Err(error.into())
        }
    };

    match result {
        Ok(()) => {
            process::exit(0);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
            process::exit(1);
        }
    }
}
