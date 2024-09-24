use tempfile::tempdir;
use tokio::fs;
use tracing::debug;
use snippy::applier::{Applier, FullContentApplier};
use snippy::extractor::Extractor;
use snippy::extractor::markdown::MarkdownExtractor;

#[tokio::test]
async fn test_complete_workflow() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = FullContentApplier::new(&base_path);

    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: file1.rs
fn main() { println!("Hello, file1!"); }
```

```rust
// filename: file2.rs
fn main() {
    println!("Hello, file2!");
}
```

```diff
--- a/file1.rs
+++ b/file1.rs
@@ -1 +1 @@
-fn main() { println!("Hello, file1!"); }
+fn main() { println!("Hello, updated file1!"); }
```

```replace
// filename: test_search_replace.rs
<<<<<<< SEARCH
=======
use std::collections::BTreeMap;
>>>>>>> REPLACE
<<<<<<< SEARCH
=======
fn main() { println!("Hello, Rust!"); }
>>>>>>> REPLACE
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 4, "Expected 4 blocks, got {}", blocks.len());

    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));
    }

    let file1_content = fs::read_to_string(base_path.join("file1.rs"))
        .await
        .unwrap_or_else(|e| panic!("Failed to read file1.rs: {:?}", e));
    let file2_content = fs::read_to_string(base_path.join("file2.rs"))
        .await
        .unwrap_or_else(|e| panic!("Failed to read file2.rs: {:?}", e));
    let search_replace_content = fs::read_to_string(base_path.join("test_search_replace.rs"))
        .await
        .unwrap_or_else(|e| panic!("Failed to read test_search_replace.rs: {:?}", e));

    assert_eq!(
        file1_content, "fn main() { println!(\"Hello, updated file1!\"); }\n",
        "File1 content mismatch"
    );
    assert_eq!(
        file2_content, "fn main() {\n    println!(\"Hello, file2!\");\n}\n",
        "File2 content mismatch"
    );
    assert_eq!(
        search_replace_content, "fn main() { println!(\"Hello, Rust!\"); }\n",
        "Content mismatch for search-replace file"
    );

    debug!("Test passed for complete workflow.");
}

#[tokio::test]
async fn test_search_replace_blocks_in_file() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = FullContentApplier::new(&base_path);

    let initial_content = "fn main() {\n    println!(\"Hello, world!\");\n}";
    let file_path = base_path.join("test.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let content = r#"
```replace
// filename: test.rs
<<<<<<< SEARCH
    println!("Hello, world!");
=======
    println!("Hello, Rust!");
>>>>>>> REPLACE
```
```replace
// filename: test.rs
<<<<<<< SEARCH
    println!("Hello, Rust!");
=======
    println!("Hello, new Rust!");
>>>>>>> REPLACE
```
    "#;

    let extractor = MarkdownExtractor::new();
    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?} for {}", e, block.content));
    }

    let content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read file: {:?}", e));

    assert_eq!(
        content,
        "fn main() {\n    println!(\"Hello, new Rust!\");\n}"
    );

    debug!("Test passed for replacing blocks in a file.");
}

#[tokio::test]
async fn test_create_new_file_with_empty_search_block() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = FullContentApplier::new(&base_path);

    let content = r#"
```replace
// filename: new_file.rs
<<<<<<< SEARCH
=======
fn main() {
    println!("This is a new file created by search-replace block.");
}
>>>>>>> REPLACE
```
    "#;

    let extractor = MarkdownExtractor::new();
    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));
    }

    let file_path = base_path.join("new_file.rs");
    let new_file_content = fs::read_to_string(&file_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read new file: {:?}", e));

    assert_eq!(
        new_file_content,
        "fn main() {\n    println!(\"This is a new file created by search-replace block.\");\n}\n"
    );

    debug!("Test passed for creating a new file with empty search block.");
}

#[tokio::test]
async fn test_delete_file_with_empty_replace_block() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = FullContentApplier::new(&base_path);

    let initial_content = "fn main() {\n    println!(\"This file will be deleted.\");\n}\n";
    let file_path = base_path.join("file_to_delete.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let content = r#"
```replace
// filename: file_to_delete.rs
<<<<<<< SEARCH
fn main() {
    println!("This file will be deleted.");
}
=======
>>>>>>> REPLACE
```
    "#;

    let extractor = MarkdownExtractor::new();
    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));
    }

    assert!(
        !file_path.exists(),
        "File should be deleted when resulting content is empty."
    );

    debug!("Test passed for deleting a file when the resulting content is empty.");
}

#[tokio::test]
async fn test_whitespace_file_deletion_with_empty_replace_block() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().to_path_buf();
    let logs_path = base_path.clone();
    let applier = FullContentApplier::new(&base_path);

    let initial_content = " \n \n";
    let file_path = base_path.join("whitespace_file_to_delete.rs");
    fs::write(&file_path, initial_content).await.unwrap();

    let content = r#"
```replace
// filename: whitespace_file_to_delete.rs
<<<<<<< SEARCH

=======
>>>>>>> REPLACE
```
    "#;

    let extractor = MarkdownExtractor::new();
    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    for block in blocks {
        applier
            .apply(&block)
            .await
            .unwrap_or_else(|e| panic!("Failed to apply content: {:?}", e));
    }

    assert!(
        !file_path.exists(),
        "File should be deleted when resulting content is an empty file with all whitespaces."
    );

    debug!("Test passed for deleting a file with all whitespaces when replace block is empty.");
}
