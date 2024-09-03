use crate::{ClipboardError, ClipboardWatcherConfig};
use arboard::{Clipboard, Error as ArboardError};
use async_trait::async_trait;
use snippy_content_extractor::{ContentApplier, ContentExtractor, MarkdownExtractor};
use std::path::PathBuf;
use tokio::{
    signal,
    time::{self, Duration},
};
use tracing::{debug, error, info, trace, warn};

#[async_trait]
pub trait ClipboardWatcher {
    async fn watch_clipboard(&self) -> Result<(), ClipboardError>;
}

pub struct BasicClipboardWatcher {
    config: ClipboardWatcherConfig,
}

impl BasicClipboardWatcher {
    pub fn new(config: ClipboardWatcherConfig) -> Self {
        BasicClipboardWatcher { config }
    }
}

#[async_trait]
impl ClipboardWatcher for BasicClipboardWatcher {
    async fn watch_clipboard(&self) -> Result<(), ClipboardError> {
        let clipboard_config = &self.config;
        let base_path = PathBuf::from(clipboard_config.watch_path.as_deref().unwrap_or("."));
        let mut clipboard = Clipboard::new()
            .map_err(|e: ArboardError| ClipboardError::ClipboardInitError(e.to_string()))?;
        let mut interval = time::interval(Duration::from_millis(clipboard_config.interval_ms));
        let mut last_content = String::new();

        let applier = ContentApplier::new(base_path, PathBuf::from("./logs"));
        let extractors: Vec<Box<dyn ContentExtractor>> = vec![Box::new(MarkdownExtractor::new())];

        debug!("Watching clipboard for new content");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    trace!("Checking clipboard content");
                    match clipboard.get_text() {
                        Ok(content) => {
                            trace!("Clipboard content length: {}", content.len());
                            if content.starts_with(&clipboard_config.first_line) {
                                trace!("Skipping self-copied content");
                                continue;
                            }

                            if content != last_content {
                                info!("New clipboard content detected");

                                let mut blocks = Vec::new();
                                for extractor in &extractors {
                                    match extractor.extract(&content) {
                                        Ok(extracted_blocks) => blocks.extend(extracted_blocks),
                                        Err(err) => {
                                            error!("Failed to extract content: {}", err);
                                            continue;
                                        }
                                    }
                                }

                                let mut files_applied_changes_to : Vec<String> = Vec::new();
                                for block in blocks {
                                    if block.filename.contains('.') == false {
                                        warn!("Skipping block with filename without extension: {}", block.filename);
                                        continue;
                                    }
                                    if let Err(err) = applier.apply(&block).await {
                                        error!("Failed to apply content: {}", err);
                                    }else{
                                        files_applied_changes_to.push(block.filename.clone());
                                    }
                                }

                                for file in files_applied_changes_to{
                                    info!("Applied changes to file: {}", file);
                                }

                                last_content = content;
                            }
                        },
                        Err(e) => {
                            error!("Failed to read clipboard content: {}", e);
                        }
                    }
                }
                _ = signal::ctrl_c() => {
                    info!("Terminating clipboard watch.");
                    break;
                }
            }
        }
        Ok(())
    }
}

pub async fn watch_clipboard(config: ClipboardWatcherConfig) -> Result<(), ClipboardError> {
    let watcher = BasicClipboardWatcher::new(config);
    watcher.watch_clipboard().await
}
