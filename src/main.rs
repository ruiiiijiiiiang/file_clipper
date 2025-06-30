use std::error::Error;

mod cli;
mod exceptions;
mod file_handler;
mod models;
mod record_handler;
mod tui;
mod utils;

use crate::cli::handle_cli;
use crate::file_handler::{handle_paste, handle_transfer};
use crate::models::{Action, Operation, RecordType};
use crate::tui::App;

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let result: Result<(), Box<dyn Error>> = match handle_cli() {
        Ok(action) => match action {
            Action::Copy(paths) => Ok(handle_transfer(paths, Operation::Copy)?),
            Action::Cut(paths) => Ok(handle_transfer(paths, Operation::Cut)?),
            Action::Paste(path) => Ok(handle_paste(path, None)?),
            Action::Clipboard => App::new(RecordType::Clipboard)?.run(),
            Action::History => App::new(RecordType::History)?.run(),
            Action::Help => {
                eprintln!("Commands: copy <path>, cut <path>, paste, list, history");
                Ok(())
            }
        },
        Err(error) => Err(Box::from(error)),
    };

    result
}
