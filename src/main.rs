use std::error::Error;

mod cli;
mod errors;
mod file_handler;
mod models;
mod records;
mod tui;
mod utils;

use {
    cli::handle_cli,
    errors::{AppError, AppInfo, AppWarning},
    file_handler::{handle_paste, handle_transfer},
    models::{Action, Operation, RecordType},
    tui::Tui,
};

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let mut app_warnings: Vec<AppWarning> = Vec::new();
    let mut infos: Vec<AppInfo> = Vec::new();

    let result: Result<(), AppError> = (|| {
        let action = handle_cli();
        match action {
            Action::Copy(paths) => {
                let (copy_infos, copy_warnings) = handle_transfer(paths, Operation::Copy)?;
                infos.extend(copy_infos);
                if let Some(warnings) = copy_warnings {
                    app_warnings.extend(warnings);
                }
            }
            Action::Cut(paths) => {
                let (cut_infos, cut_warnings) = handle_transfer(paths, Operation::Cut)?;
                infos.extend(cut_infos);
                if let Some(warnings) = cut_warnings {
                    app_warnings.extend(warnings);
                }
            }
            Action::Link(paths) => {
                let (cut_infos, cut_warnings) = handle_transfer(paths, Operation::Link)?;
                infos.extend(cut_infos);
                if let Some(warnings) = cut_warnings {
                    app_warnings.extend(warnings);
                }
            }
            Action::Paste(path) => {
                let (paste_infos, paste_warnings) = handle_paste(path, None)?;
                infos.extend(paste_infos);
                if let Some(warnings) = paste_warnings {
                    app_warnings.extend(warnings);
                }
            }
            Action::Clipboard => {
                let tui_infos = Tui::new(RecordType::Clipboard)?.run()?;
                infos.extend(tui_infos);
            }
            Action::History => {
                let tui_infos = Tui::new(RecordType::History)?.run()?;
                infos.extend(tui_infos);
            }
        }
        Ok(())
    })();

    if let Err(error) = result {
        eprintln!("[Error]: {}", error);
        #[cfg(debug_assertions)]
        eprintln!("DEBUG INFO: {:#?}", error);
        return Err(Box::from(error));
    }

    if !infos.is_empty() {
        println!("[Info]: ");
        for info in infos {
            println!("{}", info);
        }
    }

    if !app_warnings.is_empty() {
        println!("[Warning]: ");
        for warning in app_warnings {
            println!("{}", warning);
            #[cfg(debug_assertions)]
            println!("DEBUG INFO: {:#?}", warning);
        }
    }

    Ok(())
}
