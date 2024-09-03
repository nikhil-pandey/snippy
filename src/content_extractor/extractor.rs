use crate::content_extractor::block_extractor::BlockExtractor;
use crate::content_extractor::errors::ContentExtractionError;
use crate::content_extractor::parser::ParsedBlock;
use async_trait::async_trait;
use tracing::{debug, trace};

#[async_trait]
pub trait ContentExtractor: Send + Sync {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ContentExtractionError>;
}

pub struct MarkdownExtractor {
    block_extractor: BlockExtractor,
}

impl MarkdownExtractor {
    pub fn new() -> Self {
        debug!("Creating new MarkdownExtractor");
        MarkdownExtractor {
            block_extractor: BlockExtractor::new(),
        }
    }
}

impl ContentExtractor for MarkdownExtractor {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ContentExtractionError> {
        let start_time = std::time::Instant::now();
        debug!("Starting extraction of markdown content");
        let blocks = self
            .block_extractor
            .extract_blocks(content)
            .map_err(|e| ContentExtractionError::ExtractionError(e.to_string()))?;
        debug!(
            "Finished extraction of markdown content, found {} blocks",
            blocks.len()
        );
        trace!(
            "Markdown content extraction took {:?}",
            start_time.elapsed()
        );
        Ok(blocks)
    }
}
