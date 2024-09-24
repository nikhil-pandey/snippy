use snippy::content_extractor::ContentApplier;
use snippy::content_extractor::{BlockType, ParsedBlock};
use snippy::content_extractor::{ContentExtractor, MarkdownExtractor};
use tempfile::tempdir;
use tokio::fs;
use tracing::debug;

#[tokio::test]
async fn test_concurrent_diff_application() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = "fn main() { println!(\"Hello, world!\"); }\n";
    let mut handles = Vec::new();

    for i in 0..10 {
        let file_path = base_path.join(format!("test{}.rs", i));
        fs::write(&file_path, initial_content)
            .await
            .unwrap_or_else(|e| panic!("Failed to write initial content: {:?}", e));

        let block = ParsedBlock {
            filename: format!("test{}.rs", i),
            content: format!(
                r#"--- test{0}.rs
+++ test{0}.rs
@@ -1 +1 @@
-fn main() {{ println!("Hello, world!"); }}
+fn main() {{ println!("Hello, Rust!"); }}
"#,
                i
            ),
            block_type: BlockType::UnifiedDiff,
        };

        let applier = applier.clone();
        let handle = tokio::spawn(async move { applier.apply(&block).await });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(
            result.is_ok(),
            "Concurrent diff application failed: {:?}",
            result.err()
        );
    }

    for i in 0..10 {
        let file_path = base_path.join(format!("test{}.rs", i));
        let content = fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
        assert!(
            content.contains("fn main() { println!(\"Hello, Rust!\"); }\n"),
            "Content mismatch for file {}",
            i
        );
    }

    debug!("Test passed for concurrent diff application.");
}

#[tokio::test]
async fn test_concurrent_search_replace_application() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = r#"use std::collections::HashMap;
fn main() { println!("Hello, world!"); }
"#;
    let mut handles = Vec::new();

    for i in 0..10 {
        let file_path = base_path.join(format!("test_search_replace{}.rs", i));
        fs::write(&file_path, initial_content)
            .await
            .unwrap_or_else(|e| panic!("Failed to write initial content: {:?}", e));

        let block = ParsedBlock {
            filename: format!("test_search_replace{}.rs", i),
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

        let applier = applier.clone();
        let handle = tokio::spawn(async move { applier.apply(&block).await });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(
            result.is_ok(),
            "Concurrent search-replace application failed: {:?}",
            result.err()
        );
    }

    for i in 0..10 {
        let file_path = base_path.join(format!("test_search_replace{}.rs", i));
        let content = fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));
        assert_eq!(
            content,
            r#"use std::collections::BTreeMap;
fn main() { println!("Hello, Rust!"); }
"#,
            "Content mismatch for file {}",
            i
        );
    }

    debug!("Test passed for concurrent search-replace application.");
}

// #[tokio::test]
// async fn test_concurrent_extraction_from_large_files() {
//     let count = 10000;
//     let content = (0..count)
//         .map(|i| {
//             format!(
//                 "```rust\n// filename: test{}.rs\nfn main() {{ println!(\"Hello, {}!\"); }}\n```\n",
//                 i, i
//             )
//         })
//         .collect::<String>();
//
//     let mut handles = Vec::new();
//     for _ in 0..10 {
//         let extractor = MarkdownExtractor::new();
//         let content = content.clone();
//         let handle = tokio::spawn(async move { extractor.extract(&content) });
//         handles.push(handle);
//     }
//
//     for handle in handles {
//         let blocks = handle
//             .await
//             .unwrap()
//             .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
//         assert_eq!(
//             blocks.len(),
//             count,
//             "Expected {} blocks, got {}",
//             count,
//             blocks.len()
//         );
//     }
//
//     debug!("Test passed for concurrent extraction from large files.");
// }
