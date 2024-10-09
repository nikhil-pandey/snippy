use crate::errors::ClipboardError;
use crate::reporting::print_stats;
use crate::utils::{expand_patterns, format_content, read_file_content};
use arboard::Clipboard;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{PathBuf};
use tempfile::TempDir;
use tiktoken_rs::get_bpe_from_model;
use tracing::{debug, info, trace, warn};

#[derive(Debug, Clone)]
pub struct ClipboardCopierConfig {
    pub no_markdown: bool,
    pub line_number: Option<usize>,
    pub prefix: String,
    pub model: String,
    pub no_stats: bool,
    pub filename_format: String,
    pub first_line: String,
    pub xml: bool,
}

#[async_trait]
pub trait ClipboardCopier {
    async fn copy_files_to_clipboard(&self, files: Vec<String>) -> Result<(), ClipboardError>;
}

pub struct BasicClipboardCopier {
    config: ClipboardCopierConfig,
    base_path: String,
    temp_dir: Option<TempDir>,
}

impl BasicClipboardCopier {
    pub fn new(config: ClipboardCopierConfig, base_path: String, temp_dir: Option<TempDir>) -> Self {
        BasicClipboardCopier { config, base_path, temp_dir }
    }
}

#[async_trait]
impl ClipboardCopier for BasicClipboardCopier {
    async fn copy_files_to_clipboard(&self, files: Vec<String>) -> Result<(), ClipboardError> {
        let copier_config = &self.config;
        debug!("Expanding file patterns");
        let file_list =
            expand_patterns(&files).map_err(|e| ClipboardError::IoError(e.to_string()))?;
        debug!("Expanded file list: {:?}", file_list);

        debug!("Initializing clipboard");
        let mut clipboard =
            Clipboard::new().map_err(|e| ClipboardError::ClipboardInitError(e.to_string()))?;

        let mut all_content = String::new();

        let tokenizer = get_bpe_from_model(&copier_config.model)
            .map_err(|e| ClipboardError::TokenizerError(e.to_string()))?;
        let mut token_counts: HashMap<PathBuf, usize> = HashMap::new();

        // If XML formatting is enabled, wrap all file contents within a root XML element
        if copier_config.xml {
            all_content.push_str("<files>\n");
        }

        for file in file_list {
            debug!("Processing file: {}", file);
            match read_file_content(&file).await {
                Ok(content) => {
                    let mut relative_path = file.strip_prefix(&self.base_path).unwrap_or(&file);
                    relative_path = relative_path.strip_prefix("/").unwrap_or(relative_path);
                    debug!("Read content for file: {}", file);
                    let formatted_content = format_content(
                        &content,
                        &relative_path,
                        copier_config.no_markdown,
                        copier_config.line_number,
                        &copier_config.prefix,
                        copier_config.filename_format.clone(),
                        copier_config.xml,
                    )?;
                    trace!("Formatted content for file: {}", file);

                    if copier_config.xml {
                        all_content.push_str(&formatted_content);
                    } else {
                        all_content.push_str(&formatted_content);
                    }

                    if !copier_config.no_stats {
                        trace!("Encoding content to get token count for file: {}", file);
                        let tokens = tokenizer.encode_ordinary(&formatted_content);
                        let token_count = tokens.len();
                        token_counts.insert(PathBuf::from(&relative_path), token_count);
                        trace!("File {} has {} tokens", &file, token_count);
                    }
                }
                Err(e) => {
                    warn!("Failed to read file {}: {}", &file, e);
                }
            }
        }

        if copier_config.xml {
            all_content.push_str("</files>\n");
        }

        let final_content = if copier_config.xml {
            all_content
        } else {
            format!("{}{}", copier_config.first_line, all_content)
        };

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
    mut files: Vec<String>,
) -> Result<(), ClipboardError> {
    let mut temp_dir: Option<tempfile::TempDir> = None;
    let mut base_path = std::env::current_dir()
        .map_err(|e| ClipboardError::IoError(e.to_string()))?
        .to_str()
        .unwrap()
        .into();

    if files.is_empty() {
        files.push("".to_string());
    }

    if let Some(first_file) = files.get(0) {
        if is_git_url(first_file) {
            let git_url = first_file.clone();
            // Remove the git URL from the files list
            files.remove(0);

            if files.is_empty() {
                files.push("".to_string());
            }

            // Clone the repository to a temporary directory
            let tmp_dir =
                tempfile::tempdir().map_err(|e| ClipboardError::IoError(e.to_string()))?;

            use tokio::process::Command;

            info!("Cloning repository from {}", &git_url);

            let status = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg(&git_url)
                .arg(tmp_dir.path())
                .status()
                .await
                .map_err(|e| ClipboardError::CloneError(e.to_string()))?;

            if !status.success() {
                return Err(ClipboardError::CloneError(format!(
                    "Failed to clone repository from {}",
                    &git_url
                )));
            }

            // Update file paths to include the temporary directory
            let temp_dir_path = tmp_dir.path().to_path_buf();
            files = files
                .iter()
                .map(|f| {
                    let mut p = temp_dir_path.clone();
                    p.push(f);
                    p.to_string_lossy().into_owned()
                })
                .collect();
            debug!("Updated file paths: {:?}", files);

            base_path = tmp_dir.path().to_str().unwrap().into();
            temp_dir = Some(tmp_dir);
        }
    }

    let copier = BasicClipboardCopier::new(config, base_path, temp_dir);
    copier.copy_files_to_clipboard(files).await
}

fn is_git_url(url: &str) -> bool {
    url.starts_with("git@")
        || url.starts_with("https://")
        || url.starts_with("git://")
        || url.starts_with("ssh://")
}
