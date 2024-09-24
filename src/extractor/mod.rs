use crate::errors::ClipboardError;
use crate::extractor::markdown::MarkdownExtractor;
use async_trait::async_trait;
pub mod markdown;

#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub filename: String,
    pub content: String,
    pub block_type: BlockType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BlockType {
    FullContent,
    UnifiedDiff,
    SearchReplaceBlock,
}

pub trait Extractor: Send + Sync {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ClipboardError>;
}