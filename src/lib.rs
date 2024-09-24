pub mod copy;
pub mod reporting;
pub mod trie;
pub mod utils;
pub mod watch;
pub mod logger;
pub mod errors;
pub mod extractor;
pub mod applier;

pub use copy::copy_files_to_clipboard;

use clap::Parser;

