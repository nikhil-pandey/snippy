use snippy::copy::ClipboardCopierConfig;
use snippy::copy_files_to_clipboard;
use snippy::errors::ClipboardError;
use tempfile::{tempdir, TempDir};
use tokio::fs;
use tokio::process::Command;

#[tokio::test]
async fn test_copy_files_from_git_repository() -> Result<(), ClipboardError> {
    // Create a temporary directory to act as the git repository
    let repo_dir = tempdir().unwrap();
    let repo_path = repo_dir.path();

    // Initialize git repository
    let git_init_status = Command::new("git")
        .arg("init")
        .arg(".")
        .current_dir(&repo_path)
        .status()
        .await
        .expect("Failed to initialize git repository");

    assert!(
        git_init_status.success(),
        "Git init failed with status: {:?}",
        git_init_status
    );

    // Create some files
    let file1_path = repo_path.join("file1.rs");
    let file2_path = repo_path.join("file2.py");
    fs::write(
        &file1_path,
        "fn main() { println!(\"Hello from file1.rs\"); }",
    )
    .await
    .unwrap();
    fs::write(&file2_path, "print('Hello from file2.py')")
        .await
        .unwrap();

    // Add files to git
    let git_add_status = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .status()
        .await
        .expect("Failed to add files to git");

    assert!(
        git_add_status.success(),
        "Git add failed with status: {:?}",
        git_add_status
    );

    // Commit files
    let git_commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(&repo_path)
        .status()
        .await
        .expect("Failed to commit files to git");

    assert!(
        git_commit_status.success(),
        "Git commit failed with status: {:?}",
        git_commit_status
    );

    // Now, use the repository path as a file:// URL
    let git_url = format!("file://{}", repo_path.to_string_lossy());

    let copier_config = ClipboardCopierConfig {
        no_markdown: false,
        line_number: None,
        prefix: String::from("|"),
        model: String::from("gpt-4"),
        no_stats: true,
        filename_format: String::from("MarkdownHeading"),
        first_line: String::from("# Code from Git Repository\n"),
        xml: false,
    };

    let files = vec![git_url, String::from("file1.rs"), String::from("file2.py")];

    let result = copy_files_to_clipboard(copier_config, files).await;

    assert!(
        result.is_ok(),
        "copy_files_to_clipboard returned error: {:?}",
        result.err()
    );

    Ok(())
}
