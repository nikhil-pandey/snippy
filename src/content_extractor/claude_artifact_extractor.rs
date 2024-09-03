use crate::content_extractor::errors::ContentExtractionError;
use crate::content_extractor::parser::{BlockType, ParsedBlock};
use async_trait::async_trait;
use regex::Regex;
use tracing::{debug, trace};

pub struct ClaudeArtifactExtractor;

impl ClaudeArtifactExtractor {
    pub fn new() -> Self {
        debug!("Creating new ClaudeArtifactExtractor");
        ClaudeArtifactExtractor
    }

    fn extract_attributes(tag: &str) -> Vec<(String, String)> {
        let attr_regex = Regex::new(r#"(\w+)="([^"]*)"#).unwrap();
        attr_regex
            .captures_iter(tag)
            .map(|cap| (cap[1].to_string(), cap[2].to_string()))
            .collect()
    }
}

#[async_trait]
impl super::ContentExtractor for ClaudeArtifactExtractor {
    fn extract(&self, content: &str) -> Result<Vec<ParsedBlock>, ContentExtractionError> {
        let start_time = std::time::Instant::now();
        debug!("Starting extraction of Claude artifacts");

        let mut blocks = Vec::new();

        let thinking_regex = Regex::new(r"<antThinking>([\s\S]*?)</antThinking>").unwrap();
        let artifact_regex = Regex::new(r"<antArtifact([^>]*)>([\s\S]*?)</antArtifact>").unwrap();

        for cap in thinking_regex.captures_iter(content) {
            blocks.push(ParsedBlock {
                filename: "insight".to_string(),
                content: cap[1].to_string(),
                block_type: BlockType::InsightBlock,
            });
        }

        for cap in artifact_regex.captures_iter(content) {
            let attributes = Self::extract_attributes(&cap[1]);
            let filename = attributes
                .iter()
                .find(|(key, _)| key == "identifier")
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| "unnamed_artifact".to_string());

            blocks.push(ParsedBlock {
                filename,
                content: cap[2].to_string(),
                block_type: BlockType::AntArtifact,
            });
        }

        debug!(
            "Finished extraction of Claude artifacts, found {} blocks",
            blocks.len()
        );
        trace!("Claude artifact extraction took {:?}", start_time.elapsed());
        Ok(blocks)
    }
}
