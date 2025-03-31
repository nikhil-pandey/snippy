use glob::Pattern;
use std::path::Path;
use tracing::{debug, warn};

pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "target/**",
    "node_modules/**",
    ".git/**",
    "**/*.pyc",
    "**/__pycache__/**",
    ".DS_Store",
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "uv.lock",
    "dist/**",
    "build/**",
    ".venv/**",
    ".ruff_cache/**",
    ".idea/**",
    ".env",
    ".pytest_cache/**",
    ".mypy_cache/**",
    // Language-specific ignores
    // Node.js
    "pnpm-lock.yaml",
    // Ruby
    "Gemfile.lock",
    "vendor/**",
    ".bundle/**",
    // Java
    "**/*.class",
    ".gradle/**",
    // C#/.NET
    "**/bin/**",
    "**/obj/**",
    // PHP
    "vendor/**",
    "composer.lock",
    // Go
    "go.sum"
];

pub struct IgnorePatterns {
    patterns: Vec<Pattern>,
}

impl IgnorePatterns {
    pub fn new(patterns: Option<Vec<String>>) -> Self {
        let patterns_to_use = patterns.unwrap_or_else(|| 
            DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()
        );
        
        let compiled_patterns = patterns_to_use.iter()
            .filter_map(|p| match Pattern::new(p) {
                Ok(pattern) => Some(pattern),
                Err(e) => {
                    warn!("Invalid ignore pattern '{}': {}", p, e);
                    None
                }
            })
            .collect();
            
        debug!("Using ignore patterns: {:?}", patterns_to_use);
        
        IgnorePatterns { 
            patterns: compiled_patterns 
        }
    }
    
    pub fn should_ignore<P: AsRef<Path>>(&self, path: P) -> bool {
        let path_str = path.as_ref().to_string_lossy().replace("\\", "/");
        self.patterns.iter().any(|pattern| pattern.matches(&path_str))
    }
}