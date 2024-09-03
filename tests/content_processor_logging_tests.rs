use snippy::content_extractor::applier::ContentApplier;
use snippy::content_extractor::parser::{BlockType, ParsedBlock};
use tempfile::tempdir;
use tokio::fs;
use tracing::debug;

#[tokio::test]
async fn test_logging_diff_application_errors() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.join("logs");
    let applier = ContentApplier::new(base_path.clone(), logs_path.clone());

    let initial_content = "fn main() { println!(\"Hello, world!\"); }";
    let file_path = base_path.join("test.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let block = ParsedBlock {
        filename: "test.rs".to_string(),
        content: r#"--- test.rs
+++ test.rs
@@ -1 +1 @@
-fn main() { println!("Hello, world!"); }
+fn main() { println!("Hello, Rust!);
"#
        .to_string(), // Note the intentional error in the diff
        block_type: BlockType::UnifiedDiff,
    };

    let result = applier.apply(&block).await;
    assert!(result.is_err(), "Expected error, got success");

    let mut log_entries = tokio::fs::read_dir(&logs_path).await.unwrap();
    let mut count = 0;
    while let Some(_) = log_entries.next_entry().await.unwrap() {
        count += 1;
    }
    assert!(count > 0, "Expected more than 0 log entries");

    debug!("Test passed for logging diff application errors.");
}
