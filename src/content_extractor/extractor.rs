use crate::content_extractor::errors::ContentExtractionError;
use crate::content_extractor::parser::ParsedBlock;
use crate::content_extractor::BlockType;
use async_trait::async_trait;
use markdown::mdast::Node;
use markdown::mdast::Node::Code;
use markdown::{Constructs, ParseOptions};
use regex::Regex;
use tracing::{debug, trace};

#[async_trait]
pub trait ContentExtractor: Send + Sync {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ContentExtractionError>;
}

pub struct MarkdownExtractor {}

impl MarkdownExtractor {
    pub fn new() -> Self {
        debug!("Creating new MarkdownExtractor");
        MarkdownExtractor {}
    }
}

impl ContentExtractor for MarkdownExtractor {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ContentExtractionError> {
        let start_time = std::time::Instant::now();
        debug!("Starting extraction of markdown content");
        let options = ParseOptions {
            constructs: Constructs {
                code_fenced: true,
                heading_atx: true,
                heading_setext: true,
                code_indented: true,
                code_text: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        };

        let node = markdown::to_mdast(content, &options)
            .map_err(|e| ContentExtractionError::ExtractionError(e.to_string()))?;
        let mut parsed_blocks = vec![];
        let childrens = node
            .children()
            .ok_or(ContentExtractionError::ExtractionError(
                "No children found".to_string(),
            ))?;
        for (index, child) in childrens.iter().enumerate() {
            if let Code(code) = child {
                let code_value = code.value.clone() + "\n";
                if code.lang == Some("diff".to_string()) {
                    let filename_re = Regex::new(r"(?m)^\s*---\s*(.+)").expect("Invalid regex");
                    if let Some(caps) = filename_re.captures(&code.value) {
                        let filename = caps.get(1).unwrap().as_str().trim().to_string();
                        let filename = filename
                            .split_once('/')
                            .map(|(_, rest)| rest)
                            .unwrap_or(filename.as_str())
                            .to_string();
                        let block = ParsedBlock {
                            filename,
                            content: code_value,
                            block_type: BlockType::UnifiedDiff,
                        };
                        parsed_blocks.push(block);
                    }
                } else if let Some(filename) = code_value.lines().next() {
                    let filename_re = Regex::new(
                        r"(?m)^\s*(?://|#)\s*filename:\s*(.+)|^\s*/\*\s*filename:\s*(.+)\s*\*/|^\s*<!--\s*filename:\s*(.+)\s*-->"
                    ).expect("Invalid regex");
                    let block_type = if code.lang == Some("replace".to_string()) {
                        BlockType::SearchReplaceBlock
                    } else {
                        BlockType::FullContent
                    };
                    if let Some(caps) = filename_re.captures(filename) {
                        let filename = caps
                            .get(1)
                            .or_else(|| caps.get(2))
                            .or_else(|| caps.get(3))
                            .unwrap()
                            .as_str()
                            .trim()
                            .to_string();
                        let code_value = code_value
                            .split_once('\n')
                            .map(|(_, rest)| rest)
                            .unwrap_or("")
                            .to_string();
                        let block = ParsedBlock {
                            filename,
                            content: code_value,
                            block_type: block_type,
                        };
                        parsed_blocks.push(block);
                    } else {
                        if index > 0 {
                            let prev_child = &childrens[index - 1];
                            if let Node::Heading(heading) = prev_child {
                                let first_child = heading.children.last().unwrap();
                                if let Node::Text(text) = first_child {
                                    let filename = text.value.clone();
                                    let block = ParsedBlock {
                                        filename,
                                        content: code_value,
                                        block_type: block_type,
                                    };
                                    parsed_blocks.push(block);
                                } else if let Node::InlineCode(prev_code) = first_child {
                                    let filename = prev_code.value.clone();
                                    let block = ParsedBlock {
                                        filename,
                                        content: code_value,
                                        block_type: block_type,
                                    };
                                    parsed_blocks.push(block);
                                }
                            } else if let Node::Text(text) = prev_child {
                                let filename = text.value.clone();
                                let block = ParsedBlock {
                                    filename,
                                    content: code_value,
                                    block_type: block_type,
                                };
                                parsed_blocks.push(block);
                            } else if let Node::InlineCode(prev_code) = prev_child {
                                let filename = prev_code.value.clone();
                                let block = ParsedBlock {
                                    filename,
                                    content: code_value,
                                    block_type: block_type,
                                };
                                parsed_blocks.push(block);
                            }
                        }
                    }
                }
            }
        }
        // let blocks = self
        //     .block_extractor
        //     .extract_blocks(content)
        //     .map_err(|e| ContentExtractionError::ExtractionError(e.to_string()))?;
        debug!(
            "Finished extraction of markdown content, found {} blocks",
            parsed_blocks.len()
        );
        trace!(
            "Markdown content extraction took {:?}",
            start_time.elapsed()
        );
        Ok(parsed_blocks)
    }
}
