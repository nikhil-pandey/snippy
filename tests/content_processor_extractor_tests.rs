use snippy::content_extractor::extractor::{ContentExtractor, MarkdownExtractor};
use snippy::content_extractor::parser::BlockType;
use tracing::debug;

#[tokio::test]
async fn test_markdown_extractor_simple_filename_as_comment() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: path/to/file.rs
fn main() {
    println!("Hello, world!");
}
```
    "#;

    let expected_content = r#"fn main() {
    println!("Hello, world!");
}
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(blocks[0].filename, "path/to/file.rs", "Filename mismatch");
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch"
    );
    assert!(
        !blocks[0].content.contains("path/to/file.rs"),
        "Content should not contain the filename"
    );
    assert_eq!(blocks[0].content, expected_content);
    debug!("Test passed for MarkdownExtractor simple filename as comment.");
}

#[tokio::test]
async fn test_markdown_extractor_simple_filename_as_heading() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `path/to/file.rs`
```rust
fn main() {
    println!("Hello, world!");
}
```
    "#;

    let expected_content = r#"fn main() {
    println!("Hello, world!");
}
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(blocks[0].filename, "path/to/file.rs", "Filename mismatch");
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch"
    );
    assert!(
        !blocks[0].content.contains("path/to/file.rs"),
        "Content should not contain the filename"
    );
    assert_eq!(blocks[0].content, expected_content);

    debug!("Test passed for MarkdownExtractor simple filename as heading.");
}

#[tokio::test]
async fn test_markdown_extractor_simple_diff_block() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```diff
--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
-println!("Hello, world!");
+println!("Hello, Rust!");
```
    "#;

    let expected_content = r#"--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
-println!("Hello, world!");
+println!("Hello, Rust!");
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(blocks[0].filename, "test.rs", "Filename mismatch");
    assert_eq!(
        blocks[0].block_type,
        BlockType::UnifiedDiff,
        "Block type mismatch"
    );
    assert_eq!(blocks[0].content, expected_content);

    debug!("Test passed for MarkdownExtractor simple diff block.");
}

#[tokio::test]
async fn test_markdown_extractor_multiple_blocks() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: test1.rs
fn main() {
    println!("Hello, world 1!");
}
```

```rust
// filename: test2.rs
fn main() {
    println!("Hello, world 2!");
}
```

```diff
--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
-println!("Hello, world!");
+println!("Hello, Rust!");
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 3, "Expected 3 blocks, got {}", blocks.len());

    assert_eq!(
        blocks[0].filename, "test1.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch for block 0"
    );

    assert_eq!(
        blocks[1].filename, "test2.rs",
        "Filename mismatch for block 1"
    );
    assert_eq!(
        blocks[1].block_type,
        BlockType::FullContent,
        "Block type mismatch for block 1"
    );

    assert_eq!(
        blocks[2].filename, "test.rs",
        "Filename mismatch for block 2"
    );
    assert_eq!(
        blocks[2].block_type,
        BlockType::UnifiedDiff,
        "Block type mismatch for block 2"
    );

    debug!("Test passed for MarkdownExtractor multiple blocks extraction.");
}

#[tokio::test]
async fn test_markdown_extractor_no_filename() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
fn main() {
    println!("Hello, world!");
}
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 0, "Expected 0 blocks, got {}", blocks.len());

    debug!("Test passed for MarkdownExtractor no filename extraction.");
}

#[tokio::test]
async fn test_markdown_extractor_with_backticks_in_content() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: test_backticks.rs
fn main() {
    println!("Hello, world!");
    println!("This is a code block with backticks: ```code```");
}
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(blocks[0].filename, "test_backticks.rs", "Filename mismatch");
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch"
    );
    assert!(
        blocks[0].content.contains("```code```"),
        "Content missing '```code```'"
    );

    debug!("Test passed for MarkdownExtractor with backticks in content.");
}

#[tokio::test]
async fn test_markdown_extractor_mixed_blocks_with_backticks() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: test1.rs
fn main() {
    println!("Hello, world!");
}
```

```diff
--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
-println!("Hello, world!");
+println!("Hello, Rust!");
// Comment with backticks: ```comment```
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 2, "Expected 2 blocks, got {}", blocks.len());

    assert_eq!(
        blocks[0].filename, "test1.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch for block 0"
    );

    assert_eq!(
        blocks[1].filename, "test.rs",
        "Filename mismatch for block 1"
    );
    assert_eq!(
        blocks[1].block_type,
        BlockType::UnifiedDiff,
        "Block type mismatch for block 1"
    );
    assert!(
        blocks[1].content.contains("```comment```"),
        "Content missing '```comment```'"
    );

    debug!("Test passed for MarkdownExtractor mixed blocks with backticks.");
}

#[tokio::test]
async fn test_markdown_extractor_blocks_with_different_languages() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: test_rust.rs
fn main() {
    println!("Hello, Rust!");
}
```

```python
# filename: test_python.py
def main():
    print("Hello, Python!")
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 2, "Expected 2 blocks, got {}", blocks.len());

    assert_eq!(
        blocks[0].filename, "test_rust.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch for block 0"
    );

    assert_eq!(
        blocks[1].filename, "test_python.py",
        "Filename mismatch for block 1"
    );
    assert_eq!(
        blocks[1].block_type,
        BlockType::FullContent,
        "Block type mismatch for block 1"
    );

    debug!("Test passed for MarkdownExtractor blocks with different languages.");
}

#[tokio::test]
async fn test_markdown_extractor_non_standard_file_extensions() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```rust
// filename: test.customext
fn main() {
    println!("Hello, custom extension!");
}
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));
    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(blocks[0].filename, "test.customext", "Filename mismatch");
    assert_eq!(
        blocks[0].block_type,
        BlockType::FullContent,
        "Block type mismatch"
    );

    debug!("Test passed for MarkdownExtractor non-standard file extensions.");
}

#[tokio::test]
async fn test_search_replace_block_without_filename() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```replace
<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 0, "Expected 0 blocks, got {}", blocks.len());
    debug!("Test passed for MarkdownExtractor search-replace block without filename.");
}

#[tokio::test]
async fn test_search_replace_block_with_filename_comment() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
```replace
// filename: test_search_replace.rs
<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
```
    "#;

    let expected_content = r#"<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(
        blocks[0].filename, "test_search_replace.rs",
        "Filename mismatch"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch"
    );
    assert_eq!(blocks[0].content, expected_content);
    debug!("Test passed for MarkdownExtractor search-replace block with filename comment.");
}

#[tokio::test]
async fn test_search_replace_block_with_filename_heading() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `test_search_replace_heading.rs`
```replace
<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
```
    "#;

    let expected_content = r#"<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(
        blocks[0].filename, "test_search_replace_heading.rs",
        "Filename mismatch"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch"
    );
    assert_eq!(blocks[0].content, expected_content);
    debug!("Test passed for MarkdownExtractor search-replace block with filename heading.");
}

#[tokio::test]
async fn test_search_replace_block_with_additional_delimiters() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `test_additional_delimiters.rs`
```replace
<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
additional_code();
```
    "#;

    let expected_content = r#"<<<<<<< SEARCH
old_function();
=======
new_function();
>>>>>>> REPLACE
additional_code();
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(
        blocks[0].filename, "test_additional_delimiters.rs",
        "Filename mismatch"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch"
    );
    assert_eq!(blocks[0].content, expected_content);
    debug!("Test passed for MarkdownExtractor search-replace block with additional delimiters.");
}

#[tokio::test]
async fn test_search_replace_multiple_blocks_in_one_code_block() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `test_multiple_blocks.rs`
```replace
<<<<<<< SEARCH
old_function_1();
=======
new_function_1();
>>>>>>> REPLACE
<<<<<<< SEARCH
old_function_2();
=======
new_function_2();
>>>>>>> REPLACE
```
    "#;

    let expected_content = r#"<<<<<<< SEARCH
old_function_1();
=======
new_function_1();
>>>>>>> REPLACE
<<<<<<< SEARCH
old_function_2();
=======
new_function_2();
>>>>>>> REPLACE
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());
    assert_eq!(
        blocks[0].filename, "test_multiple_blocks.rs",
        "Filename mismatch"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch"
    );
    assert_eq!(blocks[0].content, expected_content);
    debug!("Test passed for MarkdownExtractor search-replace block with multiple blocks in one code block.");
}

#[tokio::test]
async fn test_search_replace_block_with_different_number_of_delimiters() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `test_incorrect_syntax.rs`
```replace
<<<<< SEARCH
old_function();
====
new_function();
>>>> REPLACE
```
    "#;

    let expected_content = r#"<<<<< SEARCH
old_function();
====
new_function();
>>>> REPLACE
"#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 1, "Expected 1 blocks, got {}", blocks.len());
    assert_eq!(
        blocks[0].filename, "test_incorrect_syntax.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 0"
    );
    assert_eq!(blocks[0].content, expected_content);
}

#[tokio::test]
async fn test_mixed_search_replace_blocks_with_headings_and_comments() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `path/tofile/first.rs`
```replace
// filename: first.rs
<<<<<<< SEARCH
fn first() {
    println!("This is the first block");
}
=======
fn first_updated() {
    println!("This is the first block updated");
}
>>>>>>> REPLACE
```

```replace
// filename: second.rs
<<<<<<< SEARCH
fn second() {
    println!("This is the second block");
}
=======
fn second_updated() {
    println!("This is the second block updated");
}
>>>>>>> REPLACE
```

### `test_no_filename.rs`
```replace
<<<<<<< SEARCH
fn no_filename() {
    println!("No filename specified in comment or heading");
}
=======
fn no_filename_updated() {
    println!("Updated block with no filename specified in comment or heading");
}
>>>>>>> REPLACE
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 3, "Expected 3 blocks, got {}", blocks.len());

    assert_eq!(
        blocks[0].filename, "first.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 0"
    );

    assert_eq!(
        blocks[1].filename, "second.rs",
        "Filename mismatch for block 1"
    );
    assert_eq!(
        blocks[1].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 1"
    );

    assert_eq!(
        blocks[2].filename, "test_no_filename.rs",
        "Filename mismatch for block 2"
    );
    assert_eq!(
        blocks[2].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 2"
    );

    debug!(
        "Test passed for MarkdownExtractor mixed search-replace blocks with headings and comments."
    );
}

#[tokio::test]
async fn test_mixed_search_replace_blocks_with_and_without_headings() {
    let extractor = MarkdownExtractor::new();
    let content = r#"
### `path/tofile/first.rs`
```replace
// filename: first.rs
<<<<<<< SEARCH
fn first() {
    println!("This is the first block");
}
=======
fn first_updated() {
    println!("This is the first block updated");
}
>>>>>>> REPLACE
```

```replace
// filename: second.rs
<<<<<<< SEARCH
fn second() {
    println!("This is the second block");
}
=======
fn second_updated() {
    println!("This is the second block updated");
}
>>>>>>> REPLACE
```

### `path/tofile/third.rs`
```replace
<<<<<<< SEARCH
fn third() {
    println!("This is the third block");
}
=======
fn third_updated() {
    println!("This is the third block updated");
}
>>>>>>> REPLACE
```
    "#;

    let blocks = extractor
        .extract(content)
        .unwrap_or_else(|e| panic!("Failed to extract content: {:?}", e));

    assert_eq!(blocks.len(), 3, "Expected 3 blocks, got {}", blocks.len());

    assert_eq!(
        blocks[0].filename, "first.rs",
        "Filename mismatch for block 0"
    );
    assert_eq!(
        blocks[0].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 0"
    );

    assert_eq!(
        blocks[1].filename, "second.rs",
        "Filename mismatch for block 1"
    );
    assert_eq!(
        blocks[1].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 1"
    );

    assert_eq!(
        blocks[2].filename, "path/tofile/third.rs",
        "Filename mismatch for block 2"
    );
    assert_eq!(
        blocks[2].block_type,
        BlockType::SearchReplaceBlock,
        "Block type mismatch for block 2"
    );

    debug!(
        "Test passed for MarkdownExtractor mixed search-replace blocks with and without headings."
    );
}
