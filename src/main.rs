use std::error::Error;

mod cli;
mod exceptions;
mod file_handler;
mod models;
mod record_handler;
mod tui;
mod utils;

use {
    cli::handle_cli,
    exceptions::{AppError, AppWarning},
    file_handler::{handle_paste, handle_transfer},
    models::{Action, Operation, RecordType},
    tui::App,
};

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let mut warnings: Vec<AppWarning> = Vec::new();
    let result: Result<(), AppError> = (|| {
        let action = handle_cli()?;
        match action {
            Action::Copy(paths) => {
                if let Some(copy_warnings) = handle_transfer(paths, Operation::Copy)? {
                    warnings.extend(copy_warnings);
                }
            }
            Action::Cut(paths) => {
                if let Some(cut_warnings) = handle_transfer(paths, Operation::Cut)? {
                    warnings.extend(cut_warnings);
                }
            }
            Action::Paste(path) => {
                if let Some(paste_warnings) = handle_paste(path, None)? {
                    warnings.extend(paste_warnings);
                }
            }
            Action::Clipboard => App::new(RecordType::Clipboard)?.run()?,
            Action::History => App::new(RecordType::History)?.run()?,
            Action::Help => {
                eprintln!("Commands: copy <path>, cut <path>, paste, list, history");
            }
        }
        Ok(())
    })();

    if let Err(error) = result {
        eprintln!("[Error]: ");
        #[cfg(debug_assertions)]
        eprintln!("{:#?}", error);
        #[cfg(not(debug_assertions))]
        eprintln!("{}", error);
        return Err(Box::from(error));
    }

    if !warnings.is_empty() {
        println!("[Warning]: ");
        for warning in warnings {
            println!("WARNING: {}", warning); // Use AppWarning's Display
            #[cfg(debug_assertions)]
            println!("  DEBUG INFO: {:#?}", warning); // Show debug for devs
        }
    }

    Ok(())
}
