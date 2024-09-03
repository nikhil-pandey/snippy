use crate::reporting::print_stats;
use crate::utils::{expand_patterns, format_content, read_file_content};
use crate::{ClipboardCopierConfig, ClipboardError};
use arboard::Clipboard;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tiktoken_rs::get_bpe_from_model;
use tracing::{debug, info, trace, warn};

#[async_trait]
pub trait ClipboardCopier {
    async fn copy_files_to_clipboard(&self, files: Vec<String>) -> Result<(), ClipboardError>;
}

pub struct BasicClipboardCopier {
    config: ClipboardCopierConfig,
}

impl BasicClipboardCopier {
    pub fn new(config: ClipboardCopierConfig) -> Self {
        BasicClipboardCopier { config }
    }
}

#[async_trait]
impl ClipboardCopier for BasicClipboardCopier {
    async fn copy_files_to_clipboard(&self, files: Vec<String>) -> Result<(), ClipboardError> {
        let copier_config = &self.config;
        debug!("Expanding file patterns");
        let file_list =
            expand_patterns(&files).map_err(|e| ClipboardError::FileReadError(e.to_string()))?;

        debug!("Initializing clipboard");
        let mut clipboard =
            Clipboard::new().map_err(|e| ClipboardError::ClipboardInitError(e.to_string()))?;

        let mut all_content = String::new();

        let tokenizer = get_bpe_from_model(&copier_config.model)
            .map_err(|e| ClipboardError::TokenizerModelError(e.to_string()))?;
        let mut token_counts: HashMap<PathBuf, usize> = HashMap::new();

        for file in file_list {
            debug!("Processing file: {}", file);
            match read_file_content(&file).await {
                Ok(content) => {
                    debug!("Read content for file: {}", file);
                    let formatted_content = format_content(
                        &content,
                        &file,
                        copier_config.no_markdown,
                        copier_config.line_number,
                        &copier_config.prefix,
                        copier_config.filename_format.clone(),
                    )?;
                    trace!("Formatted content for file: {}", file);
                    all_content.push_str(&formatted_content);

                    if !copier_config.no_stats {
                        trace!("Encoding content to get token count for file: {}", file);
                        let tokens = tokenizer.encode_ordinary(&formatted_content);
                        let token_count = tokens.len();
                        token_counts.insert(PathBuf::from(&file), token_count);
                        trace!("File {} has {} tokens", &file, token_count);
                    }
                }
                Err(e) => {
                    warn!("Failed to read file {}: {}", &file, e);
                }
            }
        }

        let final_content = format!("{}{}", copier_config.first_line, all_content);
        trace!("Final content length: {}", final_content.len());

        if !copier_config.no_stats {
            print_stats(&token_counts)?;
        }

        clipboard
            .set_text(final_content)
            .map_err(|e| ClipboardError::ClipboardWriteError(e.to_string()))?;

        info!("Files copied to clipboard successfully.");
        Ok(())
    }
}

pub async fn copy_files_to_clipboard(
    config: ClipboardCopierConfig,
    files: Vec<String>,
) -> Result<(), ClipboardError> {
    let copier = BasicClipboardCopier::new(config);
    copier.copy_files_to_clipboard(files).await
}
