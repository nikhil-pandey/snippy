use crate::applier::Applier;
use crate::errors::ClipboardError;
use crate::extractor::ParsedBlock;
use crate::utils::{read_file_async, remove_file_async, write_file_async};
use async_trait::async_trait;
use regex::Regex;
use std::path::PathBuf;
use tracing::{debug, error, info};
use crate::applier::utils::print_diff;

#[derive(Clone)]
pub struct SearchReplaceApplier {
    base_path: PathBuf,
}

impl SearchReplaceApplier {
    pub fn new(base_path: &PathBuf) -> Self {
        SearchReplaceApplier {
            base_path: base_path.clone(),
        }
    }
}

#[async_trait]
impl Applier for SearchReplaceApplier {
    async fn apply(&self, block: &ParsedBlock) -> Result<(), ClipboardError> {
        let path = self.base_path.join(&block.filename);
        debug!("Applying search-replace to file: {}", path.display());

        let search_replace_re =
            Regex::new(r"(?s)<<<+\s*SEARCH\r?\n(.*?)===+[^\r\n]*\r?\n(.*?)>>>+\s*REPLACE")
                .expect("Invalid regex");

        let original_content = read_file_async(&path).await.unwrap_or_default();
        let mut current_content = original_content.replace("\r\n", "\n");
        let block_content = block.content.replace("\r\n", "\n");

        let mut successful_replacements = 0;
        for cap in search_replace_re.captures_iter(&block_content) {
            let search_content = cap.get(1).map_or("", |m| m.as_str());
            let replace_content = cap.get(2).map_or("", |m| m.as_str());
            debug!(
                "Search: '{}'\nReplace: '{}'",
                search_content, replace_content
            );

            if search_content.trim().is_empty() {
                debug!("Empty search content, replacing with replace content");
                current_content = replace_content.to_string();
                successful_replacements += 1;
            } else if current_content.contains(search_content) {
                debug!("Found search content in file: '{}'", search_content);
                current_content = current_content.replace(search_content, replace_content);
                successful_replacements += 1;
            } else {
                let search_content_without_newline = search_content.trim_end();
                let replace_content_without_newline = replace_content.trim_end();
                if current_content.contains(search_content_without_newline) {
                    debug!(
                        "Found search content without newline in file: '{}'",
                        search_content_without_newline
                    );
                    current_content = current_content.replace(
                        search_content_without_newline,
                        replace_content_without_newline,
                    );
                    successful_replacements += 1;
                } else {
                    error!(
                        "Failed to find content to replace in file {}: '{}'",
                        path.display(),
                        search_content
                    );
                }
            }
        }

        if successful_replacements == 0 {
            return Err(ClipboardError::ContentApplicationError(format!(
                "Failed to find any content to replace in file {}",
                path.display()
            )));
        }

        if current_content.trim().is_empty() {
            remove_file_async(&path)
                .await
                .map_err(|e| ClipboardError::IoError(e.to_string()))?;
            info!("Deleted file {}", path.display());
        } else {
            write_file_async(&path, &current_content)
                .await
                .map_err(|e| ClipboardError::IoError(e.to_string()))?;
            info!("Applied search-replace to {}", path.display());
        }

        print_diff(
            &path.display().to_string(),
            &original_content,
            &current_content,
        );
        Ok(())
    }
}
