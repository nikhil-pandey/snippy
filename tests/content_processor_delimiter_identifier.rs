use snippy::content_extractor::delimiter_identifier::DelimiterIdentifier;
use snippy::content_extractor::parser::BlockType;
use tracing::debug;

#[tokio::test]
async fn test_no_filename_comment_or_heading() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```rust
    fn main() {
        println!(\"This block has no heading or filename comment\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename, None);
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_rust_code_block_with_filename_comment() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content =
        "```rust\n// filename: test.rs\nfn main() { println!(\"Hello, world!\"); }\n```\n";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("test.rs"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_heading_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/test.rs
    ```rust
    fn main() {
        println!(\"Hello, world!\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("path/tofile/test.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_heading_and_filename_comment() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/wrong_test.rs
    ```rust
    // filename: correct_test.rs
    fn main() {
        println!(\"Hello, world!\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("correct_test.rs"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_heading_with_no_filename_comment() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/no_comment.rs
    ```rust
    fn main() {
        println!(\"No filename comment available\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("path/tofile/no_comment.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_html_comment_block_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content =
        "```html\n<!-- filename: test.html -->\n<html><body>Hello, world!</body></html>\n```\n";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("test.html"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_css_comment_block_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "```css\n/* filename: test.css */\nbody { background-color: #fff; }\n```\n";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("test.css"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_python_comment_block_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "```py\n# filename: test.py\nprint(\"Hello, world!\")\n```\n";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("test.py"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_malformed_filename_comment() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "```rust\n// filename test.rs\nfn main() { println!(\"Hello, world!\"); }\n```\n";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2); // Both the start and end delimiter should be captured
    assert_eq!(delimiters[0].filename, None); // Filename should not be captured due to malformed comment
}

#[tokio::test]
async fn test_mixed_blocks_with_and_without_headings() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/first.rs
    ```rust
    fn first() {
        println!(\"This is the first block\");
    }
    ```

    ```rust
    // filename: second.rs
    fn second() {
        println!(\"This is the second block\");
    }
    ```

    ## path/tofile/third.rs
    ```rust
    fn third() {
        println!(\"This is the third block\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 6);

    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("path/tofile/first.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    assert_eq!(delimiters[2].filename.as_deref(), Some("second.rs"));
    assert_eq!(delimiters[2].block_type, BlockType::FullContent);
    assert!(delimiters[2].is_start);
    assert!(!delimiters[3].is_start);

    assert_eq!(
        delimiters[4].filename.as_deref(),
        Some("path/tofile/third.rs")
    );
    assert_eq!(delimiters[4].block_type, BlockType::FullContent);
    assert!(delimiters[4].is_start);
    assert!(!delimiters[5].is_start);
}

#[tokio::test]
async fn test_multiple_headings_with_filenames() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/first_heading.rs
    ```rust
    fn first_heading() {
        println!(\"First heading\");
    }
    ```

    ## path/tofile/second_heading.rs
    ```rust
    fn second_heading() {
        println!(\"Second heading\");
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 4);

    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("path/tofile/first_heading.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    assert_eq!(
        delimiters[2].filename.as_deref(),
        Some("path/tofile/second_heading.rs")
    );
    assert_eq!(delimiters[2].block_type, BlockType::FullContent);
    assert!(delimiters[2].is_start);
    assert!(!delimiters[3].is_start);
}

#[tokio::test]
async fn test_mixed_content() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```rust
    # filename: test.rs
    fn main() { println!(\"Hello, world!\"); }
    ```
    ```html
    <!-- filename: test.html -->
    <html>
    <body>Hello, world!</body>
    </html>
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 4);

    assert_eq!(delimiters[0].filename.as_deref(), Some("test.rs"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);

    assert!(!delimiters[1].is_start);

    assert_eq!(delimiters[2].filename.as_deref(), Some("test.html"));
    assert_eq!(delimiters[2].block_type, BlockType::FullContent);
    assert!(delimiters[2].is_start);

    assert!(!delimiters[3].is_start);
}

#[tokio::test]
async fn test_nested_blocks_with_headings_and_comments() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/outer.rs
    ```rust
    // filename: outer.rs
    fn outer() {
        println!(\"This is the outer block\");

        ## path/tofile/inner.rs
        ```rust
        // filename: inner.rs
        fn inner() {
            println!(\"This is the inner block\");
        }
        ```
    }
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 4);

    assert_eq!(delimiters[0].filename.as_deref(), Some("outer.rs"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(delimiters[1].is_start);

    assert_eq!(delimiters[1].filename.as_deref(), Some("inner.rs"));
    assert_eq!(delimiters[1].block_type, BlockType::FullContent);
    assert!(!delimiters[2].is_start);
    assert!(!delimiters[3].is_start);
}

#[tokio::test]
async fn test_nested_code_blocks_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();

    let content = r#"
    ```rust
    // filename: outer_block.rs
    fn main() {
        println!("This is the outer block");

        ```lang
        // filename: inner_block.rs
        fn inner() {
            println!("This is the inner block");
        }
        ```
    }
    ```
    "#;

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(
        delimiters.len(),
        4,
        "Expected 4 delimiters, got {}",
        delimiters.len()
    );
    assert_eq!(delimiters[0].filename.as_deref(), Some("outer_block.rs"));
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);

    assert!(delimiters[1].is_start);
    assert_eq!(delimiters[1].filename.as_deref(), Some("inner_block.rs"));
    assert_eq!(delimiters[1].block_type, BlockType::FullContent);
    assert!(!delimiters[2].is_start);

    assert!(!delimiters[3].is_start);
}

#[tokio::test]
async fn test_code_blocks_without_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();

    // Test case with a code block without a filename header
    let content = r#"
    ```rust
    // This block does not have a filename header
    fn main() {
        println!("This block should be ignored");
    }
    ```
    "#;

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(
        delimiters.len(),
        2,
        "Expected 2 delimiters, got {}",
        delimiters.len()
    );
    assert_eq!(delimiters[0].filename, None);
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_mixed_code_blocks_with_and_without_filenames() {
    let delimiter_identifier = DelimiterIdentifier::new();

    // Test case with a mix of code blocks with and without filename headers
    let content = r#"
    ```rust
    // filename: test_with_filename.rs
    fn main() {
        println!("This block should be processed");
    }
    ```

    ```rust
    // This block does not have a filename header
    fn main() {
        println!("This block should be ignored");
    }
    ```

    ```rust
    // filename: another_test_with_filename.rs
    fn main() {
        println!("This block should also be processed");
    }
    ```
    "#;

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(
        delimiters.len(),
        6,
        "Expected 6 delimiters, got {}",
        delimiters.len()
    );
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("test_with_filename.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);

    assert!(!delimiters[1].is_start);

    assert_eq!(delimiters[2].filename.as_deref(), None,);
    assert_eq!(delimiters[2].block_type, BlockType::FullContent);
    assert!(delimiters[2].is_start);
    assert!(!delimiters[3].is_start);

    assert_eq!(
        delimiters[4].filename.as_deref(),
        Some("another_test_with_filename.rs")
    );
    assert_eq!(delimiters[4].block_type, BlockType::FullContent);
    assert!(delimiters[4].is_start);
    assert!(!delimiters[5].is_start);
}

#[tokio::test]
async fn test_code_block_with_improper_filename_comment() {
    let delimiter_identifier = DelimiterIdentifier::new();

    // Test case with a code block with an improper filename comment
    let content = r#"
    ```rust
    // filename test_with_improper_comment.rs
    fn main() {
        println!("This block has an improper filename comment and should be ignored");
    }
    ```
    "#;

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(
        delimiters.len(),
        2,
        "Expected 2 delimiters, got {}",
        delimiters.len()
    );
    assert_eq!(delimiters[0].filename, None);
    assert_eq!(delimiters[0].block_type, BlockType::FullContent);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_heading_with_unified_diff_block() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ## path/tofile/unified.diff
    ```diff
    --- a/original
    +++ b/modified
    @@ -1,3 +1,3 @@
    - print(\"old line\")
    + print(\"new line\")
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("original"));
    assert_eq!(delimiters[0].block_type, BlockType::UnifiedDiff);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_diff_block_with_filepath() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```diff
    --- a/file1
    +++ b/file2
    @@ -1,3 +1,3 @@
    -old line
    +new line
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].filename.as_deref(), Some("file1"));
    assert_eq!(delimiters[0].block_type, BlockType::UnifiedDiff);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);
}

#[tokio::test]
async fn test_search_replace_block_identification_heading() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    # test_search_replace.rs
    ```replace
    <<<<<<< SEARCH
    fn main() {
        println!(\"This is a search block\");
    }
    =======
    fn main() {
        println!(\"This is a replace block\");
    }
    >>>>>>> REPLACE
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].block_type, BlockType::SearchReplaceBlock);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("test_search_replace.rs")
    );
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    debug!("Test passed for search-replace block identification.");
}

#[tokio::test]
async fn test_search_replace_block_identification_body() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```replace
    // filename: test_search_replace.rs
    <<<<<<< SEARCH
    fn main() {
        println!(\"This is a search block\");
    }
    =======
    fn main() {
        println!(\"This is a replace block\");
    }
    >>>>>>> REPLACE
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].block_type, BlockType::SearchReplaceBlock);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("test_search_replace.rs")
    );
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    debug!("Test passed for search-replace block identification.");
}

#[tokio::test]
async fn test_search_replace_block_with_filename() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```replace
    // filename: test_search_replace.rs
    <<<<<<< SEARCH
    old_function();
    =======
    new_function();
    >>>>>>> REPLACE
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("test_search_replace.rs")
    );
    assert_eq!(delimiters[0].block_type, BlockType::SearchReplaceBlock);
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    debug!("Test passed for search-replace block with filename.");
}

#[tokio::test]
async fn test_multiple_search_replace_blocks() {
    let delimiter_identifier = DelimiterIdentifier::new();
    let content = "
    ```replace
    // filename: test_search_replace.rs
    <<<<<<< SEARCH
    fn main() {
        println!(\"First search block\");
    }
    =======
    fn main() {
        println!(\"First replace block\");
    }
    >>>>>>> REPLACE

    <<<<<<< SEARCH
    fn hello() {
        println!(\"Second search block\");
    }
    =======
    fn hello() {
        println!(\"Second replace block\");
    }
    >>>>>>> REPLACE
    ```
    ";

    let delimiters = delimiter_identifier
        .identify_delimiters(content)
        .unwrap_or_else(|e| panic!("Failed to identify delimiters: {:?}", e));

    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[0].block_type, BlockType::SearchReplaceBlock);
    assert_eq!(
        delimiters[0].filename.as_deref(),
        Some("test_search_replace.rs")
    );
    assert!(delimiters[0].is_start);
    assert!(!delimiters[1].is_start);

    debug!("Test passed for multiple search-replace blocks.");
}
