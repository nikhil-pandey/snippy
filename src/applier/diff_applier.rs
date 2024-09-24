use crate::applier::Applier;
use crate::errors::ClipboardError;
use crate::extractor::ParsedBlock;
use crate::utils::{read_file_async, write_file_async};
use async_trait::async_trait;
use diffy::Patch;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct DiffApplier {
    base_path: PathBuf,
}

impl DiffApplier {
    pub fn new(base_path: &PathBuf) -> Self {
        DiffApplier {
            base_path: base_path.clone(),
        }
    }
}

#[async_trait]
impl Applier for DiffApplier {
    async fn apply(&self, block: &ParsedBlock) -> Result<(), ClipboardError> {
        let file_path = self.base_path.join(&block.filename);
        debug!("Applying diff to file: {:?}", file_path);

        let current_content = read_file_async(&file_path).await.unwrap_or_default();
        let new_content = apply_diff(&file_path, &current_content, &block.content).await?;

        write_file_async(&file_path, &new_content).await?;

        info!("Applied diff to {:?}", file_path);
        Ok(())
    }
}

pub async fn apply_diff(
    path: &PathBuf,
    current_content: &str,
    diff_content: &str,
) -> Result<String, ClipboardError> {
    let patch_result = Patch::from_str(diff_content);

    match patch_result {
        Ok(patch) => match diffy::apply(current_content, &patch) {
            Ok(new_content) => Ok(new_content),
            Err(e) => Err(ClipboardError::DiffError(format!(
                "Failed to apply diff for file {}: {}",
                path.display(),
                e.to_string()
            ))),
        },
        Err(e) => Err(ClipboardError::DiffError(format!(
            "Failed to parse diff for file {}: {}",
            path.display(),
            e.to_string()
        ))),
    }
}
