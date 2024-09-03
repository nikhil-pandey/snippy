use crate::content_extractor::errors::DiffApplicationError;
use crate::content_extractor::logger::log_diff_error;
use diffy::Patch;
use std::path::PathBuf;

pub async fn apply_diff(
    path: &PathBuf,
    current_content: &str,
    diff_content: &str,
    logs_path: &PathBuf,
) -> Result<String, DiffApplicationError> {
    let patch_result = Patch::from_str(diff_content);

    match patch_result {
        Ok(patch) => match diffy::apply(current_content, &patch) {
            Ok(new_content) => Ok(new_content),
            Err(e) => {
                log_diff_error(
                    path,
                    current_content,
                    diff_content,
                    &e.to_string(),
                    logs_path,
                )
                .await?;
                Err(DiffApplicationError::DiffApplyError(format!(
                    "Failed to apply diff for file {}: {}",
                    path.display(),
                    e.to_string()
                )))
            }
        },
        Err(e) => {
            log_diff_error(
                path,
                current_content,
                diff_content,
                &e.to_string(),
                logs_path,
            )
            .await?;
            Err(DiffApplicationError::DiffParseError(format!(
                "Failed to parse diff for file {}: {}",
                path.display(),
                e.to_string()
            )))
        }
    }
}
