use crate::applier::{Applier, DiffApplier, FullContentApplier, SearchReplaceApplier};
use crate::errors::ClipboardError;
use crate::extractor::Extractor;
use crate::utils::{read_file_async, write_file_async};
use arboard::Clipboard;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::signal;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, trace};

pub struct WatcherConfig {
    pub watch_path: PathBuf,
    pub interval_ms: u64,
    pub first_line_identifier: String,
}

pub struct ClipboardWatcher<E: Extractor + Send + Sync> {
    config: WatcherConfig,
    extractor: E,
}

impl<E: Extractor + Send + Sync> ClipboardWatcher<E> {
    pub fn new(config: WatcherConfig, extractor: E) -> Self {
        ClipboardWatcher { config, extractor }
    }

    pub async fn run(&self) -> Result<(), ClipboardError> {
        let mut clipboard =
            Clipboard::new().map_err(|e| ClipboardError::ClipboardInitError(e.to_string()))?;
        let mut interval = time::interval(Duration::from_millis(self.config.interval_ms));
        let mut last_content = String::new();

        info!("Started watching clipboard at {:?}", self.config.watch_path);

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
                                match self.extractor.extract(&content) {
                                    Ok(blocks) => {
                                        self.apply_blocks(blocks).await?;
                                        last_content = content;
                                    },
                                    Err(e) => {
                                        error!("Failed to extract content: {}", e);
                                    }
                                }
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

    async fn apply_blocks(
        &self,
        blocks: Vec<crate::extractor::ParsedBlock>,
    ) -> Result<(), ClipboardError> {
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
            } else {
                info!("Successfully applied block to {}", block.filename);
            }
        }
        Ok(())
    }
}
