use crate::applier::{Applier, DiffApplier, FullContentApplier, SearchReplaceApplier};
use crate::errors::ClipboardError;
use crate::extractor::Extractor;
use arboard::Clipboard;
use std::path::PathBuf;
use tokio::signal;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use reqwest::Client;
use std::env;
use std::collections::VecDeque;
use walkdir::WalkDir;

const MAX_HISTORY_SIZE: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
struct AIResponse {
    contains_code: bool,
    files: Vec<String>,
}

#[derive(Debug)]
struct FileHistory {
    path: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct WatcherConfig {
    pub watch_path: PathBuf,
    pub interval_ms: u64,
    pub first_line_identifier: String,
    pub ai_enabled: bool,
    pub model: String,
}

pub struct ClipboardWatcher<E: Extractor + Send + Sync> {
    config: WatcherConfig,
    extractor: E,
    modified_files: VecDeque<FileHistory>,
}

impl<E: Extractor + Send + Sync> ClipboardWatcher<E> {
    pub fn new(config: WatcherConfig, extractor: E) -> Self {
        ClipboardWatcher { 
            config, 
            extractor,
            modified_files: VecDeque::with_capacity(MAX_HISTORY_SIZE),
        }
    }

    fn add_to_history(&mut self, file_path: String) {
        if self.modified_files.len() >= MAX_HISTORY_SIZE {
            self.modified_files.pop_front();
        }
        self.modified_files.push_back(FileHistory {
            path: file_path,
            timestamp: chrono::Utc::now(),
        });
    }

    fn get_directory_tree(&self) -> Result<String, ClipboardError> {
        let mut tree = String::new();
        for entry in WalkDir::new(&self.config.watch_path)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_str().map_or(false, |s| s.starts_with('.'))) {
                match entry {
                    Ok(entry) => {
                        let path = entry.path().strip_prefix(&self.config.watch_path).unwrap_or(entry.path());
                        let depth = entry.depth();
                        let prefix = "  ".repeat(depth);
                        tree.push_str(&format!("{}{}\n", prefix, path.display()));
                    }
                    Err(e) => warn!("Error walking directory: {}", e),
                }
            }
        Ok(tree)
    }

    async fn process_with_ai(&mut self, content: &str) -> Result<(), ClipboardError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ClipboardError::ConfigError("OPENAI_API_KEY not set".to_string()))?;
        
        let client = Client::new();
        
        // Get current directory structure
        let dir_tree = self.get_directory_tree()?;
        
        // Format recent file history
        let recent_files = self.modified_files.iter()
            .map(|f| format!("{} (modified at {})", f.path, f.timestamp.format("%Y-%m-%d %H:%M:%S")))
            .collect::<Vec<_>>()
            .join("\n");

        // First call to check if content contains code blocks
        let check_prompt = format!(
            r#"Analyze the following content and determine if it contains code blocks or changes that need to be applied to files.
            
            Current working directory structure:
            {}

            Recently modified files:
            {}

            Important Notes:
            1. All file paths in your response MUST be relative to the current directory
            2. Only include files that need to be modified
            3. Make sure the files exist in the directory structure shown above

            Respond in the following JSON format:
            {{
                "contains_code": true/false,
                "files": ["relative/path/to/file1.rs", "relative/path/to/file2.rs"]
            }}
            
            Example input:
            ```rust
            fn main() {{
                println!("Hello");
            }}
            ```
            
            Example output:
            {{
                "contains_code": true,
                "files": ["src/main.rs"]
            }}
            
            Content to analyze:
            {}
            "#, 
            dir_tree,
            if recent_files.is_empty() { "No recently modified files" } else { &recent_files },
            content
        );

        info!("Sending content to OpenAI for analysis");
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": self.config.model,
                "messages": [{
                    "role": "user",
                    "content": check_prompt
                }],
                "response_format": { "type": "json_object" }
            }))
            .send()
            .await
            .map_err(|e| ClipboardError::AIError(format!("Failed to send request: {}", e)))?;

        let ai_response: serde_json::Value = response.json().await
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse response: {}", e)))?;

        let content_str = ai_response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ClipboardError::AIError("Invalid AI response format".to_string()))?;

        let parsed_response: AIResponse = serde_json::from_str(content_str)
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse AI response JSON: {}", e)))?;

        if !parsed_response.contains_code {
            info!("No code changes detected by AI");
            return Ok(());
        }

        info!("Processing {} files: {:?}", parsed_response.files.len(), parsed_response.files);

        // Process each file
        for file_path in parsed_response.files {
            let full_path = self.config.watch_path.join(&file_path);
            debug!("Processing file: {:?}", full_path);

            let current_content = fs::read_to_string(&full_path)
                .map_err(|e| ClipboardError::FileError(format!("Failed to read {}: {}", file_path, e)))?;

            let update_prompt = format!(
                r#"Update the following file content based on the provided changes.
                Important:
                1. Output ONLY the final content of the file
                2. No markdown, no backticks, just the content
                3. Preserve existing comments that are still relevant to the code
                4. Remove any temporary/instructional comments
                5. Keep documentation comments that explain functionality
                6. Maintain consistent code style with the original file
                
                Current file content:
                {}
                
                Changes to apply:
                {}
                "#,
                current_content, content
            );

            info!("Generating updated content for {}", file_path);
            let response = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&serde_json::json!({
                    "model": self.config.model,
                    "messages": [{
                        "role": "user",
                        "content": update_prompt
                    }]
                }))
                .send()
                .await
                .map_err(|e| ClipboardError::AIError(format!("Failed to send request for {}: {}", file_path, e)))?;

            let ai_response: serde_json::Value = response.json().await
                .map_err(|e| ClipboardError::AIError(format!("Failed to parse response for {}: {}", file_path, e)))?;

            let new_content = ai_response["choices"][0]["message"]["content"]
                .as_str()
                .ok_or_else(|| ClipboardError::AIError(format!("Invalid AI response format for {}", file_path)))?;

            // Create a backup of the original file
            let backup_path = full_path.with_extension("bak");
            fs::copy(&full_path, &backup_path)
                .map_err(|e| ClipboardError::FileError(format!("Failed to create backup of {}: {}", file_path, e)))?;

            // Write the new content
            fs::write(&full_path, new_content)
                .map_err(|e| ClipboardError::FileError(format!("Failed to write to {}: {}", file_path, e)))?;

            // Add to history
            self.add_to_history(file_path.clone());

            info!("Updated file: {} (backup created at {:?})", file_path, backup_path);
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ClipboardError> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| ClipboardError::ClipboardInitError(e.to_string()))?;
        let mut interval = time::interval(Duration::from_millis(self.config.interval_ms));
        let mut last_content = String::new();

        info!("Started watching clipboard at {:?}", self.config.watch_path);
        info!("AI processing is {}", if self.config.ai_enabled { "enabled" } else { "disabled" });
        if self.config.ai_enabled {
            info!("Using OpenAI model: {}", self.config.model);
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    trace!("Checking clipboard content");
                    match clipboard.get_text() {
                        Ok(content) => {
                            trace!("Clipboard content length: {}", content.len());
                            if content.starts_with(&self.config.first_line_identifier) {
                                trace!("Ignoring self-copied content to avoid recursion");
                                continue;
                            }

                            if content != last_content {
                                info!("New clipboard content detected");
                                
                                if self.config.ai_enabled {
                                    match self.process_with_ai(&content).await {
                                        Ok(_) => info!("AI processing completed successfully"),
                                        Err(e) => {
                                            error!("AI processing failed: {}", e);
                                            warn!("Falling back to standard processing");
                                            if let Err(e) = self.process_standard(&content).await {
                                                error!("Standard processing also failed: {}", e);
                                            }
                                        }
                                    }
                                } else {
                                    if let Err(e) = self.process_standard(&content).await {
                                        error!("Failed to process content: {}", e);
                                    }
                                }
                                last_content = content;
                            }
                        },
                        Err(e) => {
                            error!("Failed to read clipboard content: {}", e);
                        }
                    }
                },
                _ = signal::ctrl_c() => {
                    info!("Received Ctrl+C, terminating clipboard watcher.");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_standard(&mut self, content: &str) -> Result<(), ClipboardError> {
        match self.extractor.extract(content) {
            Ok(blocks) => {
                for block in blocks {
                    debug!("Applying block: {:?}", block);
                    let applier: Box<dyn Applier> = match block.block_type {
                        crate::extractor::BlockType::FullContent => {
                            Box::new(FullContentApplier::new(&self.config.watch_path))
                        }
                        crate::extractor::BlockType::UnifiedDiff => {
                            Box::new(DiffApplier::new(&self.config.watch_path))
                        }
                        crate::extractor::BlockType::SearchReplaceBlock => {
                            Box::new(SearchReplaceApplier::new(&self.config.watch_path))
                        }
                    };

                    if let Err(e) = applier.apply(&block).await {
                        error!("Failed to apply block: {}", e);
                        return Err(ClipboardError::ContentApplicationError(e.to_string()));
                    } else {
                        info!("Successfully applied block to {}", block.filename);
                        self.add_to_history(block.filename.clone());
                    }
                }
                Ok(())
            },
            Err(e) => Err(ClipboardError::ContentExtractionError(e.to_string()))
        }
    }
}
