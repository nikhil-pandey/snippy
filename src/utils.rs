use crate::ClipboardError;
use std::path::Path;
use tokio::fs as async_fs;
use tracing::warn;

pub fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    let normalized_path = if path.is_relative() && path.starts_with("./") {
        path.strip_prefix("./").unwrap().to_owned()
    } else {
        path.to_owned()
    };

    normalized_path.to_string_lossy().replace("\\", "/")
}

pub fn expand_patterns(patterns: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for pattern in patterns {
        let normalized_pattern = normalize_path(pattern);
        let path = Path::new(&normalized_pattern);

        if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    files.push(entry.path().to_string_lossy().to_string());
                }
            }
        } else {
            for entry in glob::glob(&normalized_pattern)? {
                match entry {
                    Ok(path) if path.is_file() => files.push(path.to_string_lossy().to_string()),
                    Err(e) => warn!("Error processing pattern {}: {:?}", pattern, e),
                    _ => {}
                }
            }
        }
    }
    Ok(files)
}

pub async fn read_file_content(file_path: &str) -> Result<String, ClipboardError> {
    async_fs::read_to_string(file_path)
        .await
        .map_err(|err| ClipboardError::FileReadError(err.to_string()))
}

pub fn format_content(
    content: &str,
    file: &str,
    no_markdown: bool,
    line_number: Option<usize>,
    prefix: &str,
    filename_format: String,
    xml: bool,
) -> Result<String, ClipboardError> {
    if xml {
        return format_xml_content(content, file, line_number);
    }

    let mut formatted_content = String::new();
    let file = normalize_path(file);
    let ext = file.split('.').last().unwrap_or("");

    match filename_format.as_str() {
        "None" => {}
        "MarkdownFirstCodeLine" => {
            if !no_markdown {
                formatted_content.push_str(&format!("```{}\n", ext));
            }
            formatted_content.push_str(&get_filename_comment(ext, &file));
        }
        "MarkdownHeading" => {
            formatted_content.push_str(&format!("### `{}`\n", file));
            if !no_markdown {
                formatted_content.push_str(&format!("```{}\n", ext));
            }
        }
        _ => {
            if !no_markdown {
                formatted_content.push_str(&format!("```{}\n", ext));
            }
        }
    }

    formatted_content.push_str(&get_line_numbered_content(content, line_number, prefix));

    if !no_markdown {
        formatted_content.push_str("```\n");
    }

    Ok(formatted_content)
}

fn format_xml_content(
    content: &str,
    file: &str,
    line_number: Option<usize>,
) -> Result<String, ClipboardError> {
    let path = normalize_path(file);
    let ext = file.split('.').last().unwrap_or("unknown");
    let file_type = get_file_type(ext)?;

    let mut formatted_content = String::new();
    formatted_content.push_str(&format!("<file path=\"{path}\" type=\"{file_type}\">\n"));

    if let Some(digits) = line_number {
        // When line numbers are provided, wrap each line in a <line> tag
        for (i, line) in content.lines().enumerate() {
            formatted_content.push_str(&format!(
                "<line number=\"{:0width$}\">",
                i + 1,
                width = digits
            ));
            formatted_content.push_str(&line);
            formatted_content.push_str("</line>\n");
        }
    } else {
        // When line numbers are not provided, output content directly
        formatted_content.push_str(&content);
        formatted_content.push('\n');
    }

    formatted_content.push_str("</file>\n");

    Ok(formatted_content)
}

fn get_file_type(ext: &str) -> Result<&'static str, ClipboardError> {
    match ext {
        // Programming languages
        "rs" => Ok("rust"),
        "py" => Ok("python"),
        "js" => Ok("javascript"),
        "ts" => Ok("typescript"),
        "tsx" => Ok("typescript"),
        "java" => Ok("java"),
        "c" => Ok("c"),
        "cpp" => Ok("cpp"),
        "h" => Ok("header"),
        "cs" => Ok("csharp"),
        "fs" => Ok("fsharp"),
        "go" => Ok("go"),
        "rb" => Ok("ruby"),
        "php" => Ok("php"),
        "swift" => Ok("swift"),
        "kt" | "kts" => Ok("kotlin"),
        "r" => Ok("r"),
        "scala" => Ok("scala"),
        "lua" => Ok("lua"),
        "dart" => Ok("dart"),

        // Web-related languages
        "html" => Ok("html"),
        "xml" => Ok("xml"),
        "xhtml" => Ok("xhtml"),
        "css" => Ok("css"),
        "scss" => Ok("scss"),
        "sass" => Ok("sass"),
        "less" => Ok("less"),

        // Scripting and configuration files
        "sh" => Ok("shell"),
        "bash" => Ok("bash"),
        "zsh" => Ok("zsh"),
        "toml" => Ok("toml"),
        "yaml" | "yml" => Ok("yaml"),
        "json" => Ok("json"),
        "ini" => Ok("ini"),
        "conf" => Ok("conf"),

        // Data formats
        "csv" => Ok("csv"),
        "tsv" => Ok("tsv"),
        "md" => Ok("markdown"),
        "rst" => Ok("reStructuredText"),

        // Markup languages
        "tex" => Ok("latex"),
        "bib" => Ok("bibtex"),

        // Miscellaneous
        "sql" => Ok("sql"),
        "bat" => Ok("batch"),
        "ps1" => Ok("powershell"),
        "dockerfile" => Ok("dockerfile"),

        // Binary files and unknown types
        "bin" => Ok("binary"),
        _ => Ok("unknown"),
    }
}

fn get_filename_comment(ext: &str, filename: &str) -> String {
    match ext {
        "rs" | "js" | "ts" | "tsx" | "java" | "c" | "cpp" | "h" | "cs" | "fs" | "json" => {
            format!("// filename: {}\n", filename)
        }
        "py" | "toml" | "sh" | "yml" | "yaml" => format!("# filename: {}\n", filename),
        "html" | "xml" => format!("<!-- filename: {} -->\n", filename),
        "css" => format!("/* filename: {} */\n", filename),
        _ => format!("// filename: {}\n", filename),
    }
}

fn get_line_numbered_content(content: &str, line_number: Option<usize>, prefix: &str) -> String {
    let mut numbered_content = String::new();
    for (i, line) in content.lines().enumerate() {
        if let Some(digits) = line_number {
            numbered_content.push_str(&format!("{:0width$}{}", i + 1, prefix, width = digits));
        }
        numbered_content.push_str(line);
        numbered_content.push('\n');
    }
    numbered_content
}
