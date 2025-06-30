use std::{io::Error as IoError, path::PathBuf};
use strum_macros::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("Failed to create configuration directory at '{path}'. Please check permissions or manually create it.")]
    CreateConfigDir {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("Failed to create record file at '{path}'. Please check permissions or disk space.")]
    CreateRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error(
        "Failed to open record file at '{path}'. It might not exist or permissions are incorrect."
    )]
    OpenRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("Failed to read record file content from '{path}'. The file might be corrupted or unreadable.")]
    ReadRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("Failed to parse data in record file at '{path}'. The file might be corrupted or malformed TOML.")]
    DeserializeRecordFile {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(
        "Failed to save data to record file. There was an internal problem serializing the data."
    )]
    SerializeRecordFile {
        #[source]
        source: toml::ser::Error,
    },
    #[error("Failed to write to record file at '{path}'. Please check disk space or permissions.")]
    WriteRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error(
        "The specified path '{path}' was not found. Please ensure it exists and is accessible."
    )]
    PathNotFound { path: PathBuf },
    #[error("Could not determine the full path for '{path}'. Check if the path is valid or if there are permission issues.")]
    AbsolutePath {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("Failed to access information about '{path}'. This might be due to incorrect permissions or a corrupted file system entry.")]
    Metadata {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("Failed to get the last modified time for '{path}'. This could be a permission issue or a problem with the file system.")]
    ModifiedAccess {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    #[error("The file type for '{path}' is not supported. This application expects a different kind of file or directory.")]
    UnsupportedType { path: PathBuf },
}

#[derive(Debug, Display)]
pub enum ValidityWarning {
    #[strum(to_string = "{0:?} was modified after last access")]
    Modified(PathBuf),
    #[strum(to_string = "{0:?} has changed in type")]
    Type(PathBuf),
    #[strum(to_string = "{0:?} has changed in size")]
    Size(PathBuf),
}

#[derive(Debug, Error)]
pub enum InputError {
    #[error("missing argument: {0}")]
    MissingArgument(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}
