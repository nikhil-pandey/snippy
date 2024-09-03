use crate::content_extractor::errors::FileOperationError;
use chrono::Utc;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tokio::fs as async_fs;
use tracing::error;
use uuid::Uuid;

#[derive(Serialize)]
struct DiagnosticInfo {
    file_path: String,
    error_message: String,
    current_content: Option<String>,
    diff_content: Option<String>,
}

pub async fn log_diff_error(
    path: &PathBuf,
    current_content: &str,
    diff_content: &str,
    error_message: &str,
    logs_path: &PathBuf,
) -> Result<(), FileOperationError> {
    let log_dir = logs_path.clone();
    async_fs::create_dir_all(&log_dir).await.map_err(|e| {
        FileOperationError::WriteError(log_dir.display().to_string(), e.to_string())
    })?;

    // Generate timestamp
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let uuid = Uuid::new_v4();
    let diagnostics_path = log_dir.join(format!(
        "{}_{}_failed_patch_diagnostics.json",
        timestamp, uuid
    ));

    let diagnostic_info = DiagnosticInfo {
        file_path: path.display().to_string(),
        error_message: error_message.to_string(),
        current_content: Some(current_content.to_string()),
        diff_content: Some(diff_content.to_string()),
    };

    let diagnostic_json = serde_json::to_string_pretty(&diagnostic_info).map_err(|e| {
        FileOperationError::WriteError(diagnostics_path.display().to_string(), e.to_string())
    })?;

    let mut diagnostics_file = File::create(&diagnostics_path).map_err(|e| {
        FileOperationError::WriteError(diagnostics_path.display().to_string(), e.to_string())
    })?;
    diagnostics_file
        .write_all(diagnostic_json.as_bytes())
        .map_err(|e| {
            FileOperationError::WriteError(diagnostics_path.display().to_string(), e.to_string())
        })?;

    error!(
        "Logged diff error diagnostics: {}",
        diagnostics_path.display()
    );

    Ok(())
}
