use crate::trie::{Trie, TrieNode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};
use crate::errors::ClipboardError;

/// Prints the statistics of token counts for files.
pub fn print_stats(token_counts: &HashMap<PathBuf, usize>) -> Result<(), ClipboardError> {
    debug!("Printing statistics for token counts");
    let mut trie = Trie::new();
    let mut total_tokens = 0;

    for (path, &token_count) in token_counts {
        trace!(
            "Inserting path into trie: {:?} with {} tokens",
            path,
            token_count
        );
        trie.insert(path, token_count)?;
        total_tokens += token_count;
    }

    info!("Overall ({} tokens)", total_tokens);
    print_tree(&trie.get_root(), "", true)?;
    Ok(())
}

fn print_tree(node: &TrieNode, prefix: &str, is_last: bool) -> Result<(), ClipboardError> {
    let connector = if is_last { "┗━━" } else { "┣━━" };

    let mut children: Vec<_> = node.children.iter().collect();
    children.sort_by(|a, b| a.0.cmp(b.0));

    for (i, (name, child)) in children.iter().enumerate() {
        let is_last_child = i == children.len() - 1;
        let new_prefix = format!("{}{}    ", prefix, if is_last_child { " " } else { "┃" });

        if child.token_count.is_some() {
            info!(
                "{}{} {} {} ({} tokens)",
                prefix,
                connector,
                get_file_icon(&Path::new(name))
                    .map_err(|e| ClipboardError::IoError(e.to_string()))?,
                name,
                child.token_count.unwrap()
            );
        } else {
            let total_tokens = child.calculate_total_tokens();
            info!(
                "{}{}📂 {} ({} tokens)",
                prefix, connector, name, total_tokens
            );
            print_tree(child, &new_prefix, is_last_child)?;
        }
    }
    Ok(())
}

pub fn get_file_icon(path: &Path) -> Result<&'static str, ClipboardError> {
    debug!("Getting file icon for path: {:?}", path);
    match path.extension().and_then(|e| e.to_str()) {
        // Programming Languages
        Some("py") | Some("pyw") | Some("pyc") | Some("pyd") | Some("pyo") | Some("pyi") => {
            Ok("🐍")
        }
        Some("js") => Ok("🟨"),
        Some("ts") | Some("tsx") => Ok("🔷"),
        Some("html") | Some("htm") => Ok("🌐"),
        Some("css") | Some("scss") | Some("sass") => Ok("🎨"),
        Some("java") | Some("class") | Some("jar") => Ok("☕"),
        Some("c") | Some("h") => Ok("🇨"),
        Some("cpp") | Some("hpp") | Some("cc") | Some("cxx") => Ok("🇨➕"),
        Some("cs") => Ok("🔷"),
        Some("go") => Ok("🐹"),
        Some("rb") | Some("erb") => Ok("💎"),
        Some("php") => Ok("🐘"),
        Some("swift") => Ok("🕊️"),
        Some("kt") | Some("kts") => Ok("🇰"),
        Some("rs") | Some("rlib") => Ok("🦀"),

        // Data and Config Files
        Some("json") => Ok("🔖"),
        Some("yaml") | Some("yml") => Ok("🗂️"),
        Some("xml") => Ok("📰"),
        Some("csv") => Ok("📊"),
        Some("ini") | Some("conf") | Some("toml") => Ok("⚙️"),
        Some("lock") => Ok("🔒"),

        // Documentation and Text
        Some("md") | Some("markdown") => Ok("📝"),
        Some("txt") => Ok("📄"),
        Some("pdf") => Ok("📕"),
        Some("doc") | Some("docx") => Ok("📘"),

        // Images
        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("svg") | Some("bmp") => {
            Ok("🖼️")
        }

        // Archives
        Some("zip") | Some("tar") | Some("gz") | Some("7z") | Some("rar") => Ok("🗜️"),

        // Executables and Scripts
        Some("exe") | Some("dll") | Some("so") | Some("dylib") => Ok("⚙️"),
        Some("sh") | Some("bash") | Some("zsh") | Some("fish") => Ok("🐚"),
        Some("bat") | Some("cmd") | Some("ps1") => Ok("🖥️"),

        // Version Control
        Some("gitignore") | Some("gitattributes") => Ok("🔒"),

        // Build and Package Management
        Some("dockerfile") => Ok("🐳"),
        Some("makefile") => Ok("🏗️"),
        Some("cmake") => Ok("🏗️"),

        // Project Files
        Some("sln") | Some("csproj") | Some("fsproj") | Some("vbproj") => Ok("🏗️"),
        Some("proj") => Ok("🏗️"),

        // .NET-specific
        Some("pdb") => Ok("🔷"),
        Some("resx") => Ok("🌐"),

        // Python-specific
        Some("ipynb") => Ok("📓"),

        // React-specific
        Some("jsx") => Ok("⚛️"),

        // TypeScript-specific
        Some("d") => Ok("📘"),

        // Fallback for directories and unknown types
        None => Ok("📁"),
        _ => Ok("📄"),
    }
}
