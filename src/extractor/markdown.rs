use super::{BlockType, Extractor, ParsedBlock};
use crate::errors::ClipboardError;
use async_trait::async_trait;
use markdown::mdast::Node;
use markdown::{to_mdast, Constructs, ParseOptions};
use regex::Regex;
use tracing::{debug, trace};

pub struct MarkdownExtractor {}

impl MarkdownExtractor {
    pub fn new() -> Self {
        debug!("Initializing MarkdownExtractor");
        MarkdownExtractor {}
    }
}

#[async_trait]
impl Extractor for MarkdownExtractor {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ClipboardError> {
        let start_time = std::time::Instant::now();
        debug!("Extracting Markdown code blocks");

        let options = ParseOptions {
            constructs: Constructs {
                heading_atx: true,
                heading_setext: true,
                code_fenced: true,
                code_indented: true,
                code_text: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        };

        let ast = to_mdast(content, &options)
            .map_err(|e| ClipboardError::ContentExtractionError(e.to_string()))?;

        let mut blocks = Vec::new();
        if let Some(children) = ast.children() {
            for (index, child) in children.iter().enumerate() {
                if let Node::Code(code_block) = child {
                    let code_content = code_block.value.trim().to_string() + "\n";
                    let language = code_block.lang.clone().unwrap_or_default();

                    let block = match language.as_str() {
                        "diff" => parse_diff_block(&code_content)?,
                        "replace" => parse_block(
                            &code_content,
                            &children,
                            index,
                            BlockType::SearchReplaceBlock,
                        )?,
                        _ => parse_block(&code_content, &children, index, BlockType::FullContent)?,
                    };

                    if let Some(block) = block {
                        blocks.push(block);
                    }
                }
            }
        }

        debug!(
            "Extraction complete. Found {} blocks in {:?}",
            blocks.len(),
            start_time.elapsed()
        );
        trace!(
            "Markdown content extraction took {:?}",
            start_time.elapsed()
        );

        Ok(blocks)
    }
}

fn parse_diff_block(content: &str) -> Result<Option<ParsedBlock>, ClipboardError> {
    let filename_regex = Regex::new(r"(?m)^\s*---\s*(.+)")?;
    for line in content.lines() {
        if let Some(caps) = filename_regex.captures(line) {
            let mut filename = caps.get(1).unwrap().as_str().trim().to_string();
            filename = filename
                .split_once('/')
                .map(|(_, rest)| rest)
                .unwrap_or(&filename)
                .to_string();
            return Ok(Some(ParsedBlock {
                filename,
                content: content.to_string(),
                block_type: BlockType::UnifiedDiff,
            }));
        }
    }
    Ok(None)
}

fn parse_block(
    content: &str,
    children: &[Node],
    index: usize,
    block_type: BlockType,
) -> Result<Option<ParsedBlock>, ClipboardError> {
    if let Some(block) = extract_block_from_content(content, &block_type)? {
        return Ok(Some(block));
    }

    if let Some(filename) = extract_filename_from_context(children, index) {
        return Ok(Some(ParsedBlock {
            filename,
            content: content.to_string(),
            block_type,
        }));
    }

    Ok(None)
}

fn extract_block_from_content(
    content: &str,
    block_type: &BlockType,
) -> Result<Option<ParsedBlock>, ClipboardError> {
    let filename_regex = Regex::new(
        r"^\s*(?://|#)\s*filename:\s*(.+)|^\s*/\*\s*filename:\s*(.+)\s*\*/|^\s*<!--\s*filename:\s*(.+)\s*-->",
    )?;

    if let Some(first_line) = content.lines().next() {
        if let Some(caps) = filename_regex.captures(first_line) {
            let filename = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .unwrap()
                .as_str()
                .trim()
                .to_string();
            let code_content = content
                .split_once('\n')
                .map(|(_, rest)| rest)
                .unwrap_or("")
                .to_string();
            return Ok(Some(ParsedBlock {
                filename,
                content: code_content,
                block_type: block_type.clone(),
            }));
        }
    }

    Ok(None)
}

fn extract_filename_from_context(children: &[Node], index: usize) -> Option<String> {
    if index > 0 {
        let prev_child = &children[index - 1];
        match prev_child {
            Node::Heading(heading) => {
                if let Some(Node::Text(text)) = heading.children.last() {
                    return Some(text.value.clone());
                } else if let Some(Node::InlineCode(code)) = heading.children.last() {
                    return Some(code.value.clone());
                }
            }
            Node::Text(text) => return Some(text.value.clone()),
            Node::InlineCode(code) => return Some(code.value.clone()),
            _ => {}
        }
    }
    None
}
