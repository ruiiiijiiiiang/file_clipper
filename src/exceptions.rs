use fs_extra::error::Error as FsError;
use glob::{GlobError, PatternError};
use std::{io::Error as IoError, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Record(#[from] RecordError),

    #[error(transparent)]
    File(#[from] FileError),
}

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

    #[error("Failed to copy '{from_path}' to '{to_path}'. This could be a permission issue or a problem with the file system.")]
    Copy {
        from_path: PathBuf,
        to_path: PathBuf,
        #[source]
        source: FsError,
    },

    #[error("Failed to move '{from_path}' to '{to_path}'. This could be a permission issue or a problem with the file system.")]
    Move {
        from_path: PathBuf,
        to_path: PathBuf,
        #[source]
        source: FsError,
    },

    #[error("Failed to read files using glob pattern '{path}'. This could be a permission issue or a problem with the file system.")]
    GlobUnreadable {
        path: PathBuf,
        #[source]
        source: GlobError,
    },

    #[error("Failed to parse glob pattern '{path}'. This might be due to an invalid pattern.")]
    GlobInvalidPattern {
        path: PathBuf,
        #[source]
        source: PatternError,
    },
}

#[derive(Debug, Error)]
pub enum AppWarning {
    #[error(transparent)]
    File(#[from] FileWarning),
    #[error(transparent)]
    Record(#[from] RecordWarning),
}

#[derive(Debug, Error)]
pub enum FileWarning {
    #[error("File '{path}' was modified since last access. Consider reviewing recent changes.")]
    ModifiedMismatch { path: PathBuf },

    #[error(
        "File '{path}' changed type from {old_type} to {new_type}. This might indicate an unexpected alteration."
    )]
    TypeMismatch {
        path: PathBuf,
        old_type: String,
        new_type: String,
    },

    #[error(
        "File '{path}' changed size from {old_size} bytes to {new_size} bytes. Check if this change was intentional."
    )]
    SizeMismatch {
        path: PathBuf,
        old_size: u64,
        new_size: u64,
    },

    #[error("Glob pattern '{path}' did not match any file.")]
    GlobUnmatched { path: PathBuf },
}

#[derive(Debug, Error)]
pub enum RecordWarning {
    #[error(
        "Failed to read record data from clipboard. This might be due to an internal error or a corrupted file."
    )]
    ClipboardUnreadable,

    #[error("Specified entry was not found in the clipboard.")]
    EntryNotFound,
}

#[derive(Debug, Error)]
pub enum InputError {
    #[error("missing argument: {0}")]
    MissingArgument(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}
