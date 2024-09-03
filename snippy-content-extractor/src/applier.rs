use crate::diff_handler::apply_diff;
use crate::errors::{DiffApplicationError, FileOperationError};
use crate::parser::{BlockType, ParsedBlock};
use std::path::PathBuf;
use tokio::fs as async_fs;
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContentApplier {
    base_path: PathBuf,
    logs_path: PathBuf,
}

impl ContentApplier {
    pub fn new(base_path: PathBuf, logs_path: PathBuf) -> Self {
        debug!(
            "Creating ContentApplier with base path: {} and logs path: {}",
            base_path.display(),
            logs_path.display()
        );
        ContentApplier {
            base_path,
            logs_path,
        }
    }

    pub async fn apply(&self, block: &ParsedBlock) -> Result<(), FileOperationError> {
        trace!("Applying block: {:?}", block);
        match block.block_type {
            BlockType::FullContent => self.apply_full_content(&block).await,
            BlockType::UnifiedDiff => self.apply_diff(&block).await,
            BlockType::SearchReplaceBlock => self.apply_search_replace(&block).await,
        }
    }

    async fn apply_full_content(&self, block: &ParsedBlock) -> Result<(), FileOperationError> {
        let path = self.base_path.join(&block.filename);
        debug!("Applying full content to file: {}", path.display());
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                FileOperationError::WriteError(parent.display().to_string(), e.to_string())
            })?;
        }

        let old_content = async_fs::read_to_string(&path).await.unwrap_or_default();

        if old_content != block.content {
            debug!("Applying new content for file: {}", path.display());
            self.print_diff(&path.display().to_string(), &old_content, &block.content)
                .await
                .map_err(|e| {
                    FileOperationError::WriteError(path.display().to_string(), e.to_string())
                })?;

            async_fs::write(&path, &block.content).await.map_err(|e| {
                FileOperationError::WriteError(path.display().to_string(), e.to_string())
            })?;
            info!("Applied full content to {}", path.display());
        } else {
            info!("No changes detected for {}", path.display());
        }
        Ok(())
    }

    async fn apply_diff(&self, block: &ParsedBlock) -> Result<(), FileOperationError> {
        let path = self.base_path.join(&block.filename);
        debug!("Applying diff to file: {}", path.display());

        let current_content = async_fs::read_to_string(&path).await.unwrap_or_default();

        let new_content = apply_diff(&path, &current_content, &block.content, &self.logs_path)
            .await
            .map_err(|e| match e {
                DiffApplicationError::DiffApplyError(msg) => {
                    FileOperationError::WriteError(path.display().to_string(), msg)
                }
                DiffApplicationError::DiffParseError(msg) => {
                    FileOperationError::WriteError(path.display().to_string(), msg)
                }
                DiffApplicationError::FileOpError(op_err) => op_err, // This will handle FileOperationError through an internal conversion.
            })?;

        self.print_diff(&path.display().to_string(), &current_content, &new_content)
            .await
            .map_err(|e| {
                FileOperationError::WriteError(path.display().to_string(), e.to_string())
            })?;

        async_fs::write(&path, &new_content).await.map_err(|e| {
            FileOperationError::WriteError(path.display().to_string(), e.to_string())
        })?;

        info!("Applied diff to {}", path.display());
        Ok(())
    }

    async fn apply_search_replace(&self, block: &ParsedBlock) -> Result<(), FileOperationError> {
        let path = self.base_path.join(&block.filename);
        debug!("Applying search-replace to file: {}", path.display());

        let search_replace_re =
            regex::Regex::new(r"(?s)<<<+\s*SEARCH\r?\n(.*?)===+[^\r\n]*\r?\n(.*?)>>>+\s*REPLACE")
                .expect("Invalid regex");

        let original_content = async_fs::read_to_string(&path).await.unwrap_or_default();
        let mut current_content = original_content.clone();
        current_content = current_content.replace("\r\n", "\n");
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
                debug!(
                    "Search without newline: '{}'\nReplace without newline: '{}'",
                    search_content_without_newline, replace_content_without_newline
                );
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
            return Err(FileOperationError::WriteError(
                path.display().to_string(),
                "No successful replacements".to_string(),
            ));
        }

        if current_content.trim().is_empty() {
            async_fs::remove_file(&path).await.map_err(|e| {
                FileOperationError::WriteError(path.display().to_string(), e.to_string())
            })?;
            info!("Deleted file {}", path.display());
        } else {
            async_fs::write(&path, &current_content)
                .await
                .map_err(|e| {
                    FileOperationError::WriteError(path.display().to_string(), e.to_string())
                })?;
            info!("Applied search-replace to {}", path.display());
        }

        self.print_diff(
            &path.display().to_string(),
            &original_content,
            &current_content,
        )
        .await
        .map_err(|e| FileOperationError::WriteError(path.display().to_string(), e.to_string()))?;

        Ok(())
    }

    async fn print_diff(&self, file: &str, old: &str, new: &str) -> Result<(), FileOperationError> {
        let patch = diffy::create_patch(old, new);
        let f = diffy::PatchFormatter::new().with_color();
        info!("Diff for file: {}\n{}", file, f.fmt_patch(&patch));
        Ok(())
    }
}
