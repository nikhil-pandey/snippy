use crate::errors::ClipboardError;
use crate::extractor::{BlockType, ParsedBlock};
use async_trait::async_trait;

pub mod diff_applier;
pub mod search_replace_applier;
pub mod full_content_applier;

pub use diff_applier::DiffApplier;
pub use search_replace_applier::SearchReplaceApplier;
pub use full_content_applier::FullContentApplier;

#[async_trait]
pub trait Applier {
    async fn apply(&self, block: &ParsedBlock) -> Result<(), ClipboardError>;
}