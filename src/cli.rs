use std::{env, path::PathBuf};

use crate::models::{Action, InputError};

pub fn handle_cli() -> Result<Action, InputError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(InputError::MissingArgument("command".to_string()));
    }

    let command = &args[1];

    match command.as_str() {
        "copy" | "cp" | "y" => {
            if args.len() < 3 {
                return Err(InputError::MissingArgument("path".to_string()));
            }
            let paths: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
            Ok(Action::Copy(paths))
        }
        "cut" | "mv" | "x" => {
            if args.len() < 3 {
                return Err(InputError::MissingArgument("path".to_string()));
            }
            let paths: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
            Ok(Action::Cut(paths))
        }
        "paste" | "p" => {
            let path = if args.len() < 3 {
                env::current_dir().unwrap()
            } else {
                PathBuf::from(&args[2])
            };
            Ok(Action::Paste(path))
        }
        "list" | "l" => Ok(Action::Clipboard),
        "history" | "h" => Ok(Action::History),
        "help" => Ok(Action::Help),
        _ => Err(InputError::InvalidCommand(command.to_string())),
    }
}
