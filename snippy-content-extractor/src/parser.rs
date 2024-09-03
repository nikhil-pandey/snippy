pub struct ParsedBlock {
    pub filename: String,
    pub content: String,
    pub block_type: BlockType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BlockType {
    FullContent,
    UnifiedDiff,
    SearchReplaceBlock,
}

impl std::fmt::Debug for ParsedBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ParsedBlock")
            .field("filename", &self.filename)
            .field("content", &self.content)
            .field("block_type", &self.block_type)
            .finish()
    }
}
