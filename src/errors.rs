use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard initialization failed: {0}")]
    ClipboardInitError(String),

    #[error("Clipboard read failed: {0}")]
    ClipboardReadError(String),

    #[error("Clipboard write failed: {0}")]
    ClipboardWriteError(String),

    #[error("Content extraction failed: {0}")]
    ContentExtractionError(String),

    #[error("Content application failed: {0}")]
    ContentApplicationError(String),

    #[error("IO Error: {0}")]
    IoError(String),

    #[error("Diff Error: {0}")]
    DiffError(String),

    #[error("Regex Error: {0}")]
    RegexError(String),

    #[error("Tokenizer Error: {0}")]
    TokenizerError(String),
}

impl From<std::io::Error> for ClipboardError {
    fn from(err: std::io::Error) -> Self {
        ClipboardError::IoError(err.to_string())
    }
}

impl From<regex::Error> for ClipboardError {
    fn from(err: regex::Error) -> Self {
        ClipboardError::RegexError(err.to_string())
    }
}
