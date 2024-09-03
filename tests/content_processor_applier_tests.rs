use snippy::content_extractor::ContentApplier;
use snippy::content_extractor::{BlockType, ParsedBlock};
use tempfile::tempdir;
use tokio::fs;
use tracing::debug;

#[tokio::test]
async fn test_content_applier_apply_full_content_to_new_file() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let block = ParsedBlock {
        filename: "new_file.rs".to_string(),
        content: "fn main() { println!(\"Hello, new file!\"); }".to_string(),
        block_type: BlockType::FullContent,
    };

    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));

    let content = fs::read_to_string(base_path.join("new_file.rs"))
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
    assert_eq!(
        content, "fn main() { println!(\"Hello, new file!\"); }",
        "File content mismatch"
    );

    debug!("Test passed for ContentApplier apply full content to new file.");
}

#[tokio::test]
async fn test_content_applier_apply_diff_with_error() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

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

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, initial_content, "Content should be unchanged");

    debug!("Test passed for ContentApplier apply diff with error.");
}

#[tokio::test]
async fn test_content_applier_apply_valid_diff() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = "fn main() { println!(\"Hello, world!\"); }\n";
    let file_path = base_path.join("test.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    // Log the initial content
    let written_content = fs::read_to_string(&file_path).await.unwrap();
    println!("Initial content written to file: {}", written_content);

    let block = ParsedBlock {
        filename: "test.rs".to_string(),
        content: r#"--- test.rs
+++ test.rs
@@ -1 +1 @@
-fn main() { println!("Hello, world!"); }
+fn main() { println!("Hello, Rust!"); }
"#
        .to_string(),
        block_type: BlockType::UnifiedDiff,
    };

    // Log the diff content
    println!("Diff content: {}", block.content);

    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply diff: {:?}", e));

    let content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
    assert_eq!(
        content, "fn main() { println!(\"Hello, Rust!\"); }\n",
        "File content mismatch"
    );

    debug!("Test passed for ContentApplier apply valid diff.");
}

#[tokio::test]
async fn test_content_applier_apply_full_content_to_existing_file_with_different_content() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = "fn main() { println!(\"Hello, world!\"); }";
    let file_path = base_path.join("existing_file.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let block = ParsedBlock {
        filename: "existing_file.rs".to_string(),
        content: "fn main() { println!(\"Hello, updated world!\"); }".to_string(),
        block_type: BlockType::FullContent,
    };

    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));

    let content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
    assert_eq!(
        content, "fn main() { println!(\"Hello, updated world!\"); }",
        "File content mismatch"
    );

    debug!("Test passed for ContentApplier apply full content to existing file with different content.");
}

#[tokio::test]
async fn test_content_applier_apply_full_content_to_existing_file_with_same_content() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = "fn main() { println!(\"Hello, world!\"); }";
    let file_path = base_path.join("existing_file.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let block = ParsedBlock {
        filename: "existing_file.rs".to_string(),
        content: initial_content.to_string(),
        block_type: BlockType::FullContent,
    };

    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));

    let content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
    assert_eq!(content, initial_content, "Content should be unchanged");

    debug!("Test passed for ContentApplier apply full content to existing file with same content.");
}

#[tokio::test]
async fn test_content_applier_apply_search_replace_block_success() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = r#"use std::collections::HashMap;
fn main() { println!("Hello, world!"); }
"#;
    let file_path = base_path.join("test_search_replace.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let block = ParsedBlock {
        filename: "test_search_replace.rs".to_string(),
        content: r#"<<<<<<< SEARCH
use std::collections::HashMap;
=======
use std::collections::BTreeMap;
>>>>>>> REPLACE
<<<<<<< SEARCH
fn main() { println!("Hello, world!"); }
=======
fn main() { println!("Hello, Rust!"); }
>>>>>>> REPLACE
"#
        .to_string(),
        block_type: BlockType::SearchReplaceBlock,
    };

    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply search-replace block: {:?}", e));

    let content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
    assert_eq!(
        content,
        r#"use std::collections::BTreeMap;
fn main() { println!("Hello, Rust!"); }
"#,
        "File content mismatch"
    );

    debug!("Test passed for ContentApplier apply search-replace block successfully.");
}

#[tokio::test]
async fn test_content_applier_apply_search_replace_block_fail() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = "fn main() { println!(\"Hello, world!\"); }";
    let file_path = base_path.join("test_search_replace_fail.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let block = ParsedBlock {
        filename: "test_search_replace_fail.rs".to_string(),
        content: r#"<<<<<<< SEARCH
non_existent_function();
=======
replacement_function();
>>>>>>> REPLACE
"#
        .to_string(),
        block_type: BlockType::SearchReplaceBlock,
    };

    let result = applier.apply(&block).await;
    assert!(result.is_err(), "Expected error, got success");

    debug!("Test passed for ContentApplier apply search-replace block with failure.");
}
