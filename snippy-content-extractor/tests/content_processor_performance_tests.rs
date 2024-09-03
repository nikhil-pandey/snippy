use snippy_content_extractor::applier::ContentApplier;
use snippy_content_extractor::delimiter_identifier::DelimiterIdentifier;
use snippy_content_extractor::extractor::{ContentExtractor, MarkdownExtractor};
use snippy_content_extractor::parser::{BlockType, ParsedBlock};
use std::time::Instant;
use tempfile::tempdir;
use tokio::fs;
use tracing::debug;

#[tokio::test]
// #[tracing_test::traced_test]
async fn test_large_file_extraction_performance() {
    let count = 10000;
    let extractor = MarkdownExtractor::new();
    let content = (0..count)
        .map(|i| {
            format!(
                "```rust\n// filename: test{}.rs\nfn main() {{ println!(\"Hello, {}!\"); }}\n```\n",
                i, i
            )
        })
        .collect::<String>();

    let start = std::time::Instant::now();
    let blocks = extractor
        .extract(&content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    let duration = start.elapsed();

    assert_eq!(
        blocks.len(),
        count,
        "Expected {} blocks, got {}",
        count,
        blocks.len()
    );
    assert!(
        duration.as_secs() < 10,
        "Extraction took too long: {:?}",
        duration
    );

    debug!("Test passed for large file extraction performance.");
}

#[tokio::test]
async fn test_delimiter_identifier_performance() {
    let count = 10000;
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = (0..count)
        .map(|i| {
            format!(
                "```rust\n// filename: test{}.rs\nfn main() {{ println!(\"Hello, {}!\"); }}\n```\n",
                i, i
            )
        })
        .collect::<String>();

    let start = Instant::now();
    let delimiters = delimiter_identifier
        .identify_delimiters(&content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));
    let duration = start.elapsed();

    assert_eq!(
        delimiters.len(),
        count * 2, // Each block should have a start and end delimiter
        "Expected {} delimiters, got {}",
        count * 2,
        delimiters.len()
    );
    assert!(
        duration.as_secs() < 10,
        "Delimiter identification took too long: {:?}",
        duration
    );

    debug!("Test passed for DelimiterIdentifier performance.");
}

#[tokio::test]
async fn test_large_file_apply_full_content_performance() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let blocks: Vec<ParsedBlock> = (0..1000)
        .map(|i| ParsedBlock {
            filename: format!("test{}.rs", i),
            content: format!("fn main() {{ println!(\"Hello, {}!\"); }}", i),
            block_type: BlockType::FullContent,
        })
        .collect();

    let start = Instant::now();
    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));
    }
    let duration = start.elapsed();

    assert!(
        duration.as_secs() < 60,
        "Application took too long: {:?}",
        duration
    );

    debug!("Test passed for large file apply full content performance.");
}

#[tokio::test]
async fn test_large_diff_apply_performance() {
    let count = 10_000;
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = ContentApplier::new(base_path.clone(), logs_path);

    let initial_content = (0..count)
        .map(|i| format!("fn main() {{ println!(\"Hello, {}!\"); }}\n", i))
        .collect::<String>();
    let file_path = base_path.join("large_test.rs");
    fs::write(&file_path, &initial_content).await.unwrap();

    // Generate the diff content with correct hunk headers
    let mut diff_content = String::new();
    let mut current_line = 1; // Start from line 1

    for i in 0..count {
        let old_line = format!("fn main() {{ println!(\"Hello, {}!\"); }}\n", i);
        let new_line = format!("fn main() {{ println!(\"Hello, updated {}!\"); }}\n", i);

        // Create the hunk header
        let hunk_header = format!("@@ -{},1 +{},1 @@\n", current_line, current_line);
        diff_content.push_str(&hunk_header);
        diff_content.push_str(&format!("-{}", old_line));
        diff_content.push_str(&format!("+{}", new_line));
        current_line += 1;
    }

    let block = ParsedBlock {
        filename: "large_test.rs".to_string(),
        content: format!("--- large_test.rs\n+++ large_test.rs\n{}", diff_content),
        block_type: BlockType::UnifiedDiff,
    };

    let start = Instant::now();
    applier
        .apply(&block)
        .await
        .unwrap_or_else(|e| panic!("Failed to apply diff: {:?}", e));
    let duration = start.elapsed();

    assert!(
        duration.as_secs() < 90,
        "Application took too long: {:?}",
        duration
    );

    debug!("Test passed for large diff apply performance.");
}
