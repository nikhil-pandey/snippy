use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileOperationError {
    #[error("Failed to read file {0}: {1}")]
    ReadError(String, String),
    #[error("Failed to write file {0}: {1}")]
    WriteError(String, String),
}

#[derive(Error, Debug)]
pub enum DiffApplicationError {
    #[error("Error parsing diff: {0}")]
    DiffParseError(String),
    #[error("Error applying diff: {0}")]
    DiffApplyError(String),
    #[error(transparent)]
    FileOpError(#[from] FileOperationError),
}

#[derive(Error, Debug)]
pub enum ContentExtractionError {
    #[error("Error extracting content: {0}")]
    ExtractionError(String),
    #[error(transparent)]
    FormatError(#[from] ContentFormatError),
}

#[derive(Error, Debug)]
pub enum ContentFormatError {
    #[error("Error formatting content: {0}")]
    FormatError(String),
}
