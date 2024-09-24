use crate::content_extractor::errors::ContentFormatError;
use crate::content_extractor::parser::BlockType;
use regex::Regex;
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug)]
pub struct DelimitedBlock {
    pub start_index: usize,
    pub content_start: usize,
    pub is_start: bool,
    pub filename: Option<String>,
    pub block_type: BlockType,
}

#[derive(Debug)]
pub struct DelimiterIdentifier {
    start_delimiter_re: Regex,
    end_delimiter_re: Regex,
    heading_re: Regex,
    filename_re: Regex,
    diff_file_re: Regex,
}

impl DelimiterIdentifier {
    pub fn new() -> Self {
        let start_delimiter_re = Regex::new(r"(?m)^\s*```(\w+)\s*$").expect("Invalid regex");
        let end_delimiter_re = Regex::new(r"(?m)^\s*```\s*$").expect("Invalid regex");
        let heading_re = Regex::new(r"(?m)^\s*#{1,}\s*`?([^`\s]+)`?").expect("Invalid regex");
        let filename_re = Regex::new(
            r"(?m)^\s*(?://|#)\s*filename:\s*(.+)|^\s*/\*\s*filename:\s*(.+)\s*\*/|^\s*<!--\s*filename:\s*(.+)\s*-->"
        ).expect("Invalid regex");
        let diff_file_re = Regex::new(r"(?m)^\s*---\s*(.+)").expect("Invalid regex");
        DelimiterIdentifier {
            start_delimiter_re,
            end_delimiter_re,
            heading_re,
            filename_re,
            diff_file_re,
        }
    }

    pub fn identify_delimiters(
        &self,
        content: &str,
    ) -> Result<Vec<DelimitedBlock>, ContentFormatError> {
        let mut delimiters = vec![];
        let re_start_time = std::time::Instant::now();

        // Extract headings
        let mut headings = vec![];
        for cap in self.heading_re.captures_iter(content) {
            headings.push((
                cap.get(0).unwrap().start(),
                cap.get(1).unwrap().as_str().to_string(),
            ));
        }

        // Extract filenames
        let mut filenames = HashMap::new();
        for cap in self.filename_re.captures_iter(content) {
            let start_index = cap.get(0).unwrap().start();
            let filename = cap
                .get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .map(|m| m.as_str().trim().to_string());
            if let Some(filename) = filename {
                filenames.insert(start_index, filename);
            }
        }

        // Extract diff block filenames
        for cap in self.diff_file_re.captures_iter(content) {
            let start_index = cap.get(0).unwrap().start();
            let filename = cap.get(1).map(|m| m.as_str().trim().to_string());
            if let Some(filename) = filename {
                filenames.insert(start_index, filename);
            }
        }

        // Apply start delimiter regex on the entire content
        for cap in self.start_delimiter_re.captures_iter(content) {
            let start_index = cap.get(0).unwrap().start();
            let end_index = cap.get(0).unwrap().end();
            let language = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let is_start = !language.is_empty();
            let block_type = if language == "diff" {
                BlockType::UnifiedDiff
            } else if language == "replace" {
                BlockType::SearchReplaceBlock
            } else {
                BlockType::FullContent
            };

            // Determine the content start index
            let filename_index = content[end_index..]
                .find('\n')
                .map(|pos| end_index + pos + 1)
                .unwrap_or(end_index);
            let content_start = if filenames.contains_key(&filename_index)
                && block_type != BlockType::UnifiedDiff
            {
                content[filename_index..]
                    .find('\n')
                    .map(|pos| filename_index + pos + 1)
                    .unwrap_or(filename_index)
            } else if content[end_index..].starts_with('\n') {
                end_index + 1
            } else if content[end_index..].starts_with("\r\n") {
                end_index + 2
            } else {
                end_index
            };

            let filename = filenames.get(&filename_index).cloned().or_else(|| {
                headings
                    .iter()
                    .rev()
                    .find(|&&(pos, _)| {
                        pos < start_index
                            && pos
                                >= delimiters
                                    .last()
                                    .map(|d: &DelimitedBlock| d.start_index)
                                    .unwrap_or(0)
                    })
                    .map(|&(_, ref fname)| fname.clone())
            });
            let filename = filename.map(|fname| {
                if fname.starts_with("a/") || fname.starts_with("b/") {
                    fname[2..].to_string()
                } else {
                    fname
                }
            });

            delimiters.push(DelimitedBlock {
                start_index,
                content_start,
                is_start,
                filename,
                block_type,
            });
        }

        // Apply end delimiter regex on the entire content
        for cap in self.end_delimiter_re.captures_iter(content) {
            let start_index = cap.get(0).unwrap().start();
            let end_index = cap.get(0).unwrap().end();
            delimiters.push(DelimitedBlock {
                start_index,
                content_start: end_index,
                is_start: false,
                filename: None,
                block_type: BlockType::FullContent,
            });
        }

        delimiters.sort_by_key(|d| d.start_index);
        debug!(
            "Delimiter identification took {:?}",
            re_start_time.elapsed()
        );

        if delimiters.is_empty() {
            return Err(ContentFormatError::FormatError(
                "No delimiters found".to_string(),
            ));
        }
        Ok(delimiters)
    }
}
