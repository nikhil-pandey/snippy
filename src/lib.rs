pub mod copy;
pub mod reporting;
pub mod trie;
pub mod utils;
pub mod watch;
pub mod content_extractor;

pub use copy::copy_files_to_clipboard;
pub use watch::watch_clipboard;

use clap::Parser;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("Failed to initialize clipboard: {0}")]
    ClipboardInitError(String),
    #[error("Failed to write to clipboard: {0}")]
    ClipboardWriteError(String),
    #[error("File read error: {0}")]
    FileReadError(String),
    #[error("Content format error: {0}")]
    FormatContentError(String),
    #[error("Tokenizer model error: {0}")]
    TokenizerModelError(String),
    #[error("Error printing statistics: {0}")]
    PrintStatsError(String),
}

#[derive(Debug, Clone)]
pub struct ClipboardCopierConfig {
    pub no_markdown: bool,
    pub line_number: Option<usize>,
    pub prefix: String,
    pub model: String,
    pub no_stats: bool,
    pub filename_format: String,
    pub first_line: String,
}

#[derive(Debug, Clone, Parser)]
pub struct ClipboardWatcherConfig {
    pub watch_path: Option<String>,
    pub interval_ms: u64,
    pub first_line: String,
}
