use crate::applier::Applier;
use crate::errors::ClipboardError;
use crate::extractor::ParsedBlock;
use crate::utils::{read_file_async, write_file_async};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info};

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

        write_file_async(&file_path, &block.content).await?;

        info!("Applied full content to {:?}", file_path);
        Ok(())
    }
}
