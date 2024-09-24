use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, trace};
use crate::errors::ClipboardError;

/// Represents a node in a trie.
pub struct TrieNode {
    pub children: HashMap<String, TrieNode>,
    pub token_count: Option<usize>,
}

impl TrieNode {
    pub fn new() -> Self {
        debug!("Creating new TrieNode");
        TrieNode {
            children: HashMap::new(),
            token_count: None,
        }
    }

    /// Calculates the total number of tokens in the subtree.
    pub fn calculate_total_tokens(&self) -> usize {
        trace!("Calculating total tokens for TrieNode");
        self.token_count.unwrap_or(0)
            + self
            .children
            .values()
            .map(|child| child.calculate_total_tokens())
            .sum::<usize>()
    }
}

/// Represents a trie data structure.
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        debug!("Creating new Trie");
        Trie {
            root: TrieNode::new(),
        }
    }

    /// Inserts a path with its token count into the trie.
    pub fn insert(&mut self, path: &Path, token_count: usize) -> Result<(), ClipboardError> {
        debug!(
            "Inserting path: {:?} with token count: {}",
            path, token_count
        );
        let mut current_node = &mut self.root;
        for component in path.iter() {
            let component_str = component.to_string_lossy().into_owned();
            current_node = current_node
                .children
                .entry(component_str)
                .or_insert_with(TrieNode::new);
        }
        if current_node.token_count.is_some() {
            trace!("Overwriting existing token count for path: {:?}", path);
        }
        current_node.token_count = Some(token_count);
        Ok(())
    }

    /// Returns the root node of the trie.
    pub fn get_root(&self) -> &TrieNode {
        trace!("Getting root of the Trie");
        &self.root
    }
}
