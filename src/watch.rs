use crate::applier::{Applier, DiffApplier, FullContentApplier, SearchReplaceApplier};
use crate::errors::ClipboardError;
use crate::extractor::Extractor;
use crate::ignore::{DEFAULT_IGNORE_PATTERNS, IgnorePatterns};
use crate::llm::{LLMClient, TokenUsage, MODEL_PRICING};
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
use std::collections::HashMap;
use futures::future::join_all;

const MAX_HISTORY_SIZE: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
struct AIResponse {
    contains_code: bool,
    files: Vec<String>,
}

#[derive(Debug)]
struct FileHistory {
    path: String,
}

#[derive(Debug)]
struct ProcessingStats {
    file_path: String,
    total_time: Duration,
    llm_response_time: Duration,
    io_time: Duration,
    token_usage: TokenUsage,
}

#[derive(Clone)]
pub struct WatcherConfig {
    pub watch_path: PathBuf,
    pub interval_ms: u64,
    pub first_line_identifier: String,
    pub ai_enabled: bool,
    pub model: String,
    pub ignore_patterns: Vec<String>,
    pub predictions_enabled: bool,
    pub store_enabled: bool,
    pub metadata: HashMap<String, String>,
    pub one_shot: bool,
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
            predictions_enabled: false,
            store_enabled: false,
            metadata: {
                let mut m = HashMap::new();
                m.insert("tool".to_string(), "snippy".to_string());
                m
            },
            one_shot: false,
        }
    }
}

pub struct ClipboardWatcher<E: Extractor + Send + Sync> {
    config: WatcherConfig,
    extractor: E,
    modified_files: VecDeque<FileHistory>,
    llm_client: LLMClient,
    total_token_usage: TokenUsage,
    ignore_patterns: IgnorePatterns,
}

impl<E: Extractor + Send + Sync> ClipboardWatcher<E> {
    pub fn new(config: WatcherConfig, extractor: E) -> Self {
        ClipboardWatcher { 
            llm_client: LLMClient::new(
                config.model.clone(),
                config.store_enabled,
                config.metadata.clone()
            ),
            config: config.clone(), 
            extractor,
            modified_files: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            total_token_usage: TokenUsage::default(),
            ignore_patterns: IgnorePatterns::new(Some(config.ignore_patterns)),
        }
    }

    fn should_ignore(&self, path: &str) -> bool {
        self.ignore_patterns.should_ignore(path)
    }

    fn add_to_history(&mut self, file_path: String) {
        if self.modified_files.len() >= MAX_HISTORY_SIZE {
            self.modified_files.pop_front();
        }
        self.modified_files.push_back(FileHistory {
            path: file_path,
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
            .map(|f| f.path.clone())
            .collect::<Vec<_>>()
            .join("\n");

        info!("Starting AI analysis of clipboard content ({} characters)", content.len());
        let check_prompt = format!(
            r#"Analyze the following content and determine if it contains code blocks or changes that need to be applied to files.
            This is from the user's clipboard. A lot of times the user can copy things that are not code, or are not relevant to the current project.
            Make sure to return true only if the content contains code that needs to be applied to files.
            
            Current working directory structure:
            {}

            Recently modified files:
            {}

            Important Notes:
            1. All file paths in your response MUST be relative to the current directory
            2. Only include files that need to be modified
            3. Make sure the files exist in the directory structure shown above or will be created as new files

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
            content
        );

        info!("Analyzing content with AI");
        match self.llm_client.call_with_json_response::<AIResponse>(&check_prompt).await {
            Ok((parsed_response, usage, analysis_time)) => {
                info!("Analysis token usage: {} (response time: {:?})", usage.format_details(&self.config.model), analysis_time);
                self.update_token_usage(usage);

                if !parsed_response.contains_code {
                    info!("AI analysis: No code changes detected in clipboard content");
                    return Ok(());
                }

                let files: Vec<_> = parsed_response.files.into_iter()
                    .filter(|f| !self.should_ignore(f))
                    .collect();

                if files.is_empty() {
                    info!("AI analysis: All detected files are in ignored paths");
                    return Ok(());
                }

                info!("AI analysis: Found {} files to process", files.len());
                for (i, file) in files.iter().enumerate() {
                    info!("File {}/{}: {}", i + 1, files.len(), file);
                }

                // Process files in parallel
                let mut futures = Vec::new();
                let total_files = files.len();
                
                // Convert files to a Vec of owned Strings to avoid borrowing issues
                let files: Vec<String> = files.into_iter().collect();
                
                // Create owned versions of all data needed in the async tasks
                let content = content.to_string();
                let watch_path = self.config.watch_path.clone();
                let model = self.config.model.clone();
                let predictions_enabled = self.config.predictions_enabled;
                let store_enabled = self.config.store_enabled;
                let metadata = self.config.metadata.clone();

                for (i, file_path) in files.into_iter().enumerate() {
                    let content = content.clone();
                    let watch_path = watch_path.clone();
                    let model = model.clone();
                    let metadata = metadata.clone();

                    futures.push(tokio::spawn(async move {
                        let file_start_time = Instant::now();
                        let mut io_time = Duration::default();
                        let full_path = watch_path.join(&file_path);
                        info!("Processing file {}/{}: {}", i + 1, total_files, file_path);
                        debug!("Processing file: {:?}", full_path);

                        let io_start = Instant::now();
                        let (current_content, is_new_file) = if full_path.exists() {
                            (fs::read_to_string(&full_path)
                                .map_err(|e| ClipboardError::FileError(format!("Failed to read {}: {}", file_path, e)))?, false)
                        } else {
                            info!("File {} does not exist, will be created as new", file_path);
                            (String::new(), true)
                        };
                        io_time += io_start.elapsed();

                        let update_prompt = format!(
                            r#"Update or create the following file based on the provided changes.
                            Important:
                            1. Output ONLY the final content of the file
                            2. No markdown, no backticks, just the content i.e. code and comments
                            3. Think carefully about which comments to preserve:
                               - Keep meaningful documentation and code explanation comments
                               - Remove any temporary/instructional comments like "// ... rest of the code ..." or "// ... rest of the file ..." or "// remove this"
                               - Remove any comments that were meant to guide you
                               - Follow any guidelines provided as code comments in the changes section
                            4. When deciding what changes to apply:
                               - ONLY apply changes that are meant for this specific file denoted by the filename below
                               - Ignore any changes meant for other files
                               - Don't remove code that isn't being modified by the changes
                               - Think about the context and purpose of each section
                            5. Maintain consistent code style with the original file unless the new changes suggest otherwise or have better style
                            
                            Changes to apply (ONLY apply changes meant for this file):
                            <changes>
                            {}
                            </changes>

                            File name: 
                            <filename>
                            {}
                            </filename>
                            
                            File status:
                            <status>
                            {}
                            </status>
                            
                            Current content:
                            <current_content>
                            {}
                            </current_content>
                            
                            "#,
                            // content first to take advantage of cached tokens
                            content,
                            file_path,
                            if is_new_file { "NEW FILE" } else { "EXISTING FILE" },
                            current_content,
                        );

                        info!("Generating updated content for {}", file_path);
                        let prediction = if predictions_enabled && !is_new_file {
                            Some(current_content.as_str())
                        } else {
                            None
                        };

                        let llm_client = LLMClient::new(model.clone(), store_enabled, metadata);
                        let response = llm_client.call(&update_prompt, prediction).await?;

                        info!("Generation token usage: {} (response time: {:?})", 
                            response.usage.format_details(&model), 
                            response.response_time
                        );

                        let io_start = Instant::now();
                        print_diff(&file_path, &current_content, &response.content);

                        if let Some(parent) = full_path.parent() {
                            if !parent.exists() {
                                fs::create_dir_all(parent)
                                    .map_err(|e| ClipboardError::FileError(format!("Failed to create directories for {}: {}", file_path, e)))?;
                            }
                        }

                        fs::write(&full_path, &response.content)
                            .map_err(|e| ClipboardError::FileError(format!("Failed to write to {}: {}", file_path, e)))?;
                        io_time += io_start.elapsed();

                        Ok::<_, ClipboardError>(ProcessingStats {
                            file_path: file_path.clone(),
                            total_time: file_start_time.elapsed(),
                            llm_response_time: response.response_time,
                            io_time,
                            token_usage: response.usage,
                        })
                    }));
                }

                // Wait for all futures to complete
                let results = join_all(futures).await;
                
                // Process results
                for result in results {
                    match result {
                        Ok(Ok(stats)) => {
                            self.add_to_history(stats.file_path.clone());
                            self.update_token_usage(stats.token_usage);
                            processing_stats.push(stats);
                        }
                        Ok(Err(e)) => {
                            error!("Error processing file: {}", e);
                            return Err(e);
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            return Err(ClipboardError::TaskJoinError(e.to_string()));
                        }
                    }
                }

                let total_time = start_time.elapsed();
                let total_llm_time: Duration = processing_stats.iter()
                    .map(|s| s.llm_response_time)
                    .sum();
                let total_io_time: Duration = processing_stats.iter()
                    .map(|s| s.io_time)
                    .sum();

                info!("Processing completed in {:?}", total_time);
                info!("Time breakdown:");
                info!("  LLM time: {:?}", total_llm_time);
                info!("  IO time: {:?}", total_io_time);
                info!("  Other time: {:?}", {
                    if total_time > total_llm_time + total_io_time {
                        total_time - total_llm_time - total_io_time
                    } else {
                        Duration::from_secs(0)
                    }
                });
                info!("Files processed:");
                for stats in processing_stats {
                    info!("  {} (total={:?}, llm={:?}, io={:?}, tokens: {})",
                        stats.file_path,
                        stats.total_time,
                        stats.llm_response_time,
                        stats.io_time,
                        stats.token_usage.format_details(&self.config.model)
                    );
                }

                Ok(())
            }
            Err(ClipboardError::Cancelled(_)) => {
                info!("Analysis cancelled by user");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn run(&mut self) -> Result<(), ClipboardError> {
        // Check for OpenAI API key if AI is enabled
        if self.config.ai_enabled {
            if std::env::var("OPENAI_API_KEY").is_err() {
                error!("OpenAI API key not found in environment. Please set OPENAI_API_KEY environment variable.");
                return Err(ClipboardError::ConfigError("OPENAI_API_KEY environment variable not set".to_string()));
            }
        }

        let mut clipboard = Clipboard::new()
            .map_err(|e| ClipboardError::ClipboardInitError(e.to_string()))?;
        
        // Get initial clipboard content
        let content = clipboard.get_text()
            .map_err(|e| ClipboardError::ClipboardReadError(e.to_string()))?;
            
        if content.is_empty() {
            error!("Clipboard is empty");
            return Err(ClipboardError::ClipboardReadError("Clipboard is empty".to_string()));
        }

        // Initialize last_content with current clipboard content to ignore initial state
        let mut last_content = content.clone();
        
        let mut interval = time::interval(Duration::from_millis(self.config.interval_ms));
        
        info!("Started {} clipboard at {:?}", 
            if self.config.one_shot { "one-shot processing" } else { "watching" },
            self.config.watch_path
        );
        info!("AI processing is {}", if self.config.ai_enabled { "enabled" } else { "disabled" });
        
        if self.config.ai_enabled {
            info!("Using OpenAI model: {} (input=${:.3}/1M, cached=${:.3}/1M, output=${:.3}/1M)", 
                self.config.model,
                MODEL_PRICING.get(self.config.model.as_str())
                    .map_or(0.0, |p| p.input_price),
                MODEL_PRICING.get(self.config.model.as_str())
                    .map_or(0.0, |p| p.cached_price),
                MODEL_PRICING.get(self.config.model.as_str())
                    .map_or(0.0, |p| p.output_price)
            );
            info!("Predictions are {}", if self.config.predictions_enabled { "enabled" } else { "disabled" });
            if self.config.store_enabled {
                info!("Data storage is enabled with metadata: {:?}", self.config.metadata);
            }
        }

        let start_time = Instant::now();

        if self.config.one_shot {
            // For one-shot mode, process immediately and return
            if self.config.ai_enabled {
                self.process_with_ai(&content).await?;
            } else {
                self.process_standard(&content).await?;
            }
            info!("One-shot processing completed in {:?}", start_time.elapsed());
            if self.config.ai_enabled {
                info!("Total token usage: {}", 
                    self.total_token_usage.format_details(&self.config.model)
                );
            }
            return Ok(());
        }

        // Watch mode loop
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
                                        Ok(_) => {
                                            info!("AI processing completed successfully");
                                        }
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
                    info!("Total token usage: {}", 
                        self.total_token_usage.format_details(&self.config.model)
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
