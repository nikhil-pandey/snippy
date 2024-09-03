use crate::content_extractor::delimiter_identifier::DelimiterIdentifier;
use crate::content_extractor::errors::ContentExtractionError;
use crate::content_extractor::parser::ParsedBlock;
use tracing::{instrument, trace, warn};

#[derive(Debug)]
pub struct BlockExtractor {
    delimiter_identifier: DelimiterIdentifier,
}

impl BlockExtractor {
    pub fn new() -> Self {
        BlockExtractor {
            delimiter_identifier: DelimiterIdentifier::new(),
        }
    }

    #[instrument(level = "trace", skip(self, content))]
    pub fn extract_blocks(
        &self,
        content: &str,
    ) -> Result<Vec<ParsedBlock>, ContentExtractionError> {
        let start_time = std::time::Instant::now();
        let mut blocks = Vec::new();
        let delimiters = self.delimiter_identifier.identify_delimiters(content)?;
        trace!("Delimiter identification took {:?}", start_time.elapsed());

        let mut block_time = std::time::Instant::now();
        let mut stack = vec![];
        for delimiter in delimiters {
            if delimiter.is_start {
                stack.push(delimiter);
                block_time = std::time::Instant::now();
            } else if let Some(starting_delimiter) = stack.pop() {
                if stack.is_empty() {
                    let start_index = starting_delimiter.content_start;
                    let end_index = delimiter.start_index;
                    let block_content = content[start_index..end_index].to_string();

                    if let Some(ref fname) = starting_delimiter.filename {
                        let block = ParsedBlock {
                            filename: fname.clone(),
                            content: block_content,
                            block_type: starting_delimiter.block_type,
                        };
                        blocks.push(block);
                    } else {
                        continue;
                    }

                    trace!("Block extraction took {:?}", block_time.elapsed());
                }
            }
        }

        if !stack.is_empty() {
            for unclosed_delimiter in stack {
                warn!(
                    "Unclosed block detected starting at index {}. Ignoring the incomplete block.",
                    unclosed_delimiter.start_index
                );
            }
        }

        trace!("Overall block extraction took {:?}", start_time.elapsed());
        Ok(blocks)
    }
}
