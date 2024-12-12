use crate::applier::{Applier, DiffApplier, FullContentApplier, SearchReplaceApplier};
use crate::errors::ClipboardError;
use crate::extractor::Extractor;
use crate::llm::{LLMClient, TokenUsage};
use crate::applier::utils::print_diff;
use arboard::Clipboard;
use std::path::PathBuf;
use tokio::signal;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::VecDeque;
use walkdir::WalkDir;
use std::time::Instant;
use glob::Pattern;

const MAX_HISTORY_SIZE: usize = 10;
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "target/**",
    "node_modules/**",
    ".git/**",
    "**/*.pyc",
    "**/__pycache__/**",
    ".DS_Store",
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "dist/**",
    "build/**",
];

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

#[derive(Debug)]
struct ProcessingStats {
    file_path: String,
    processing_time: Duration,
    token_usage: TokenUsage,
}

pub struct WatcherConfig {
    pub watch_path: PathBuf,
    pub interval_ms: u64,
    pub first_line_identifier: String,
    pub ai_enabled: bool,
    pub model: String,
    pub ignore_patterns: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: PathBuf::from("."),
            interval_ms: 1000,
            first_line_identifier: "# Relevant Code".to_string(),
            ai_enabled: false,
            model: "gpt-4o-mini".to_string(),
            ignore_patterns: DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

pub struct ClipboardWatcher<E: Extractor + Send + Sync> {
    config: WatcherConfig,
    extractor: E,
    modified_files: VecDeque<FileHistory>,
    llm_client: LLMClient,
    total_token_usage: TokenUsage,
    ignore_patterns: Vec<Pattern>,
}

impl<E: Extractor + Send + Sync> ClipboardWatcher<E> {
    pub fn new(config: WatcherConfig, extractor: E) -> Self {
        let ignore_patterns = config.ignore_patterns.iter()
            .filter_map(|p| match Pattern::new(p) {
                Ok(pattern) => Some(pattern),
                Err(e) => {
                    warn!("Invalid ignore pattern '{}': {}", p, e);
                    None
                }
            })
            .collect();

        ClipboardWatcher { 
            llm_client: LLMClient::new(config.model.clone()),
            config, 
            extractor,
            modified_files: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            total_token_usage: TokenUsage::default(),
            ignore_patterns,
        }
    }

    fn should_ignore(&self, path: &str) -> bool {
        self.ignore_patterns.iter().any(|pattern| pattern.matches(path))
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

    fn update_token_usage(&mut self, usage: TokenUsage) {
        self.total_token_usage = self.total_token_usage + usage;
    }

    fn get_directory_tree(&self) -> Result<String, ClipboardError> {
        let mut tree = String::new();
        for entry in WalkDir::new(&self.config.watch_path)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| {
                !e.file_name().to_str().map_or(false, |s| s.starts_with('.')) && 
                !e.path().strip_prefix(&self.config.watch_path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map_or(false, |p| self.should_ignore(p))
            }) {
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
        let start_time = Instant::now();
        let mut processing_stats = Vec::new();
        
        let dir_tree = self.get_directory_tree()?;
        
        let recent_files = self.modified_files.iter()
            .map(|f| format!("{} (modified at {})", f.path, f.timestamp.format("%Y-%m-%d %H:%M:%S")))
            .collect::<Vec<_>>()
            .join("\n");

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
            4. Ignore the following patterns: {}

            Respond in the following JSON format:
            {{
                "contains_code": true/false,
                "files": ["relative/path/to/file1.rs", "relative/path/to/file2.rs"]
            }}
            
            Content to analyze:
            {}
            "#, 
            dir_tree,
            if recent_files.is_empty() { "No recently modified files" } else { &recent_files },
            self.config.ignore_patterns.join(", "),
            content
        );

        info!("Analyzing content with AI");
        let (parsed_response, usage) = self.llm_client.call_with_json_response::<AIResponse>(&check_prompt).await?;
        self.update_token_usage(usage);

        if !parsed_response.contains_code {
            info!("No code changes detected by AI");
            return Ok(());
        }

        // Filter out ignored files
        let files: Vec<_> = parsed_response.files.into_iter()
            .filter(|f| !self.should_ignore(f))
            .collect();

        if files.is_empty() {
            info!("All detected files are in ignored paths");
            return Ok(());
        }

        info!("Processing {} files: {:?}", files.len(), files);

        // Process each file
        for file_path in files {
            let file_start_time = Instant::now();
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
            let response = self.llm_client.call(&update_prompt).await?;
            self.update_token_usage(response.usage);

            // Print diff before applying changes
            print_diff(&file_path, &current_content, &response.content);

            // Write the new content
            fs::write(&full_path, &response.content)
                .map_err(|e| ClipboardError::FileError(format!("Failed to write to {}: {}", file_path, e)))?;

            // Add to history and stats
            self.add_to_history(file_path.clone());
            processing_stats.push(ProcessingStats {
                file_path,
                processing_time: file_start_time.elapsed(),
                token_usage: response.usage,
            });
        }

        // Print final stats
        info!("Processing completed in {:?}", start_time.elapsed());
        info!("Files processed:");
        for stats in processing_stats {
            info!("  {} (took {:?}, tokens: prompt={}, completion={}, total={})",
                stats.file_path,
                stats.processing_time,
                stats.token_usage.prompt_tokens,
                stats.token_usage.completion_tokens,
                stats.token_usage.total_tokens
            );
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

        let start_time = Instant::now();

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
                    info!("Total runtime: {:?}", start_time.elapsed());
                    info!("Total token usage: prompt={}, completion={}, total={}", 
                        self.total_token_usage.prompt_tokens,
                        self.total_token_usage.completion_tokens,
                        self.total_token_usage.total_tokens
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_standard(&mut self, content: &str) -> Result<(), ClipboardError> {
        let start_time = Instant::now();
        let mut files_processed = Vec::new();

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

                    let file_start_time = Instant::now();
                    if let Err(e) = applier.apply(&block).await {
                        error!("Failed to apply block: {}", e);
                        return Err(ClipboardError::ContentApplicationError(e.to_string()));
                    } else {
                        info!("Successfully applied block to {}", block.filename);
                        self.add_to_history(block.filename.clone());
                        files_processed.push((block.filename, file_start_time.elapsed()));
                    }
                }

                info!("Standard processing completed in {:?}", start_time.elapsed());
                info!("Files processed:");
                for (file, duration) in files_processed {
                    info!("  {} (took {:?})", file, duration);
                }

                Ok(())
            },
            Err(e) => Err(ClipboardError::ContentExtractionError(e.to_string()))
        }
    }
}
