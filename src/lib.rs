pub mod applier;
pub mod copy;
pub mod errors;
pub mod extractor;
pub mod logger;
pub mod reporting;
pub mod trie;
pub mod utils;
pub mod watch;

pub use copy::copy_files_to_clipboard;

use clap::Parser;
