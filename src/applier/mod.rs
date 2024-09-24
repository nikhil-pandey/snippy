use crate::errors::ClipboardError;
use crate::extractor::ParsedBlock;
use async_trait::async_trait;

pub mod diff_applier;
pub mod full_content_applier;
pub mod search_replace_applier;

pub use diff_applier::DiffApplier;
pub use full_content_applier::FullContentApplier;
pub use search_replace_applier::SearchReplaceApplier;

#[async_trait]
pub trait Applier {
    async fn apply(&self, block: &ParsedBlock) -> Result<(), ClipboardError>;
}
