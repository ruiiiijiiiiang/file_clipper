use std::{error::Error, process};

mod cli;
mod file_handler;
mod models;
mod record_handler;
mod tui;
mod utils;

use crate::cli::handle_cli;
use crate::file_handler::{handle_paste, handle_transfer};
use crate::models::{Action, Operation, RecordType};
use crate::tui::enter_tui_mode;

fn main() {
    let result: Result<(), Box<dyn Error>> = match handle_cli() {
        Ok(action) => match action {
            Action::Copy(paths) => handle_transfer(paths, Operation::Copy),
            Action::Cut(paths) => handle_transfer(paths, Operation::Cut),
            Action::Paste(path) => handle_paste(path, None),
            Action::Clipboard => enter_tui_mode(RecordType::Clipboard),
            Action::History => enter_tui_mode(RecordType::History),
            Action::Help => {
                eprintln!("Commands: copy <path>, cut <path>, paste, list, history");
                Ok(())
            }
        },
        Err(error) => Err(Box::from(error)),
    };

    match result {
        Ok(()) => {
            process::exit(0);
        }
        Err(error) => {
            eprintln!("[Error]: {}", error);
            process::exit(1);
        }
    }
}
