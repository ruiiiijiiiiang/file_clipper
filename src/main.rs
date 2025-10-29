use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::{error::Error, io, str::FromStr};

mod cli;
mod errors;
mod files;
mod models;
mod records;
mod tui;

#[cfg(test)]
pub mod test_helpers;

use {
    cli::{Cli, handle_cli},
    errors::{AppError, AppInfo, AppWarning},
    files::{handle_paste, handle_transfer},
    models::{Action, Operation, RecordType},
    records::clear_records,
    tui::Tui,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "completions" {
        let shell = if args.len() > 2 {
            Shell::from_str(args[2].as_str()).expect(
                "Invalid shell provided; possible values: [bash, elvish, fish, powershell, zsh]",
            )
        } else {
            Shell::from_env().unwrap_or(Shell::Bash)
        };
        generate(shell, &mut Cli::command(), "clp", &mut io::stdout());
        return Ok(());
    }

    color_eyre::install()?;
    let mut app_warnings: Vec<AppWarning> = Vec::new();
    let mut app_infos: Vec<AppInfo> = Vec::new();

    let result: Result<(), AppError> = (|| {
        let action = handle_cli();
        match action {
            Action::Copy(paths) => {
                let (copy_infos, copy_warnings) = handle_transfer(paths, Operation::Copy)?;
                app_infos.extend(copy_infos);
                app_warnings.extend(copy_warnings);
            }
            Action::Cut(paths) => {
                let (cut_infos, cut_warnings) = handle_transfer(paths, Operation::Cut)?;
                app_infos.extend(cut_infos);
                app_warnings.extend(cut_warnings);
            }
            Action::Link(paths) => {
                let (link_infos, link_warnings) = handle_transfer(paths, Operation::Link)?;
                app_infos.extend(link_infos);
                app_warnings.extend(link_warnings);
            }
            Action::Paste(path) => {
                let (paste_infos, paste_warnings) = handle_paste(path, None)?;
                app_infos.extend(paste_infos);
                app_warnings.extend(paste_warnings);
            }
            Action::Clipboard => {
                let (tui_infos, tui_warnings) = Tui::new(RecordType::Clipboard)?.run()?;
                app_infos.extend(tui_infos);
                app_warnings.extend(tui_warnings);
            }
            Action::History => {
                let (tui_infos, tui_warnings) = Tui::new(RecordType::History)?.run()?;
                app_infos.extend(tui_infos);
                app_warnings.extend(tui_warnings);
            }
            Action::Clear => {
                let clear_infos = clear_records()?;
                app_infos.extend(clear_infos);
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

    if !app_infos.is_empty() {
        println!("[Info]: ");
        for info in app_infos {
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
