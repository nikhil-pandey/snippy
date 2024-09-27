use crate::applier::Applier;
use crate::errors::ClipboardError;
use crate::extractor::ParsedBlock;
use crate::utils::{read_file_async, write_file_async};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info};
use crate::applier::utils::print_diff;

#[derive(Clone)]
pub struct FullContentApplier {
    base_path: PathBuf,
}

impl FullContentApplier {
    pub fn new(base_path: &PathBuf) -> Self {
        FullContentApplier {
            base_path: base_path.clone(),
        }
    }
}

#[async_trait]
impl Applier for FullContentApplier {
    async fn apply(&self, block: &ParsedBlock) -> Result<(), ClipboardError> {
        let file_path = self.base_path.join(&block.filename);
        debug!("Applying full content to file: {:?}", file_path);
        let original_content = read_file_async(&file_path).await
            .unwrap_or_default();
        write_file_async(&file_path, &block.content).await?;
        print_diff(
            &block.filename,
            &original_content,
            &block.content,
        );
        info!("Applied full content to {:?}", file_path);
        Ok(())
    }
}
