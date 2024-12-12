use clap::{Parser, Subcommand};
use snippy::copy::ClipboardCopierConfig;
use snippy::copy_files_to_clipboard;
use snippy::extractor::markdown::MarkdownExtractor;
use snippy::logger::initialize_logger;
use snippy::watch::{ClipboardWatcher, WatcherConfig};
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    cmd: SubCommands,
}

#[derive(Subcommand, Debug, Clone)]
enum SubCommands {
    Copy(CopyArgs),
    Watch(WatchArgs),
}

#[derive(Parser, Debug, Clone)]
struct CopyArgs {
    #[arg(required = true)]
    files: Vec<String>,
    #[arg(short = 'm', long, default_value = "false")]
    no_markdown: bool,
    #[arg(short = 'l', long, default_value = None)]
    line_number: Option<usize>,
    #[arg(short = 'p', long, default_value = "|")]
    prefix: String,
    #[arg(short = 'M', long, default_value = "gpt-4o")]
    model: String,
    #[arg(short = 's', long, default_value = "false")]
    no_stats: bool,
    #[arg(long, default_value = "MarkdownHeading")]
    filename_format: Option<String>,
    #[arg(long, default_value = "# Relevant Code\n")]
    pub first_line: String,
    #[arg(long, help = "Format the output as XML")]
    pub xml: bool,
}

#[derive(Parser, Debug, Clone)]
struct WatchArgs {
    #[arg(short = 'x', long)]
    watch_path: Option<String>,
    #[arg(short = 'i', long, default_value_t = 1000)]
    interval_ms: u64,
    #[arg(long, default_value = "# Relevant Code")]
    pub first_line: String,
    #[arg(long)]
    pub ai: bool,
    #[arg(long, default_value = "gpt-4o-mini")]
    pub model: String,
    #[arg(long = "ignore", help = "Patterns to ignore (e.g., 'target/**', '**/*.pyc')")]
    pub ignore_patterns: Option<Vec<String>>,
    #[arg(long, help = "Enable predictions for faster responses when files exist")]
    pub predictions: bool,
    #[arg(long, help = "Store data in OpenAI for future use")]
    pub store: bool,
    #[arg(long = "metadata", help = "Additional metadata key-value pairs (format: key=value, e.g., --metadata project=snippy --metadata env=dev)")]
    pub metadata: Option<Vec<String>>,
    #[arg(long, help = "Run once and exit instead of watching continuously")]
    pub once: bool,
}

#[tokio::main]
async fn main() {
    let cli_args = CliArgs::parse();
    initialize_logger();

    match cli_args.cmd {
        SubCommands::Copy(args) => {
            let copier_config = ClipboardCopierConfig {
                no_markdown: args.no_markdown,
                line_number: args.line_number,
                prefix: args.prefix.clone(),
                model: args.model.clone(),
                no_stats: args.no_stats,
                filename_format: args
                    .filename_format
                    .clone()
                    .unwrap_or_else(|| "None".to_owned()),
                first_line: args.first_line,
                xml: args.xml,
            };
            if let Err(e) = copy_files_to_clipboard(copier_config, args.files).await {
                eprintln!("Error copying files to clipboard: {}", e);
            }
        }
        SubCommands::Watch(args) => {
            info!("Starting Clipboard Watcher");

            let mut watcher_config = WatcherConfig::default();
            watcher_config.interval_ms = args.interval_ms;
            watcher_config.watch_path = PathBuf::from(args.watch_path.unwrap_or_else(|| ".".to_owned()));
            watcher_config.first_line_identifier = args.first_line;
            watcher_config.ai_enabled = args.ai;
            watcher_config.model = args.model;
            watcher_config.predictions_enabled = args.predictions;
            watcher_config.store_enabled = args.store;
            
            // Parse metadata
            let mut metadata = HashMap::new();
            metadata.insert("tool".to_string(), "snippy".to_string());
            if let Some(meta_args) = args.metadata {
                for kv in meta_args {
                    match kv.split_once('=') {
                        Some((key, value)) => {
                            if key.trim().is_empty() {
                                warn!("Ignoring metadata with empty key: {}", kv);
                            } else if value.trim().is_empty() {
                                warn!("Ignoring metadata with empty value: {}", kv);
                            } else {
                                metadata.insert(key.trim().to_string(), value.trim().to_string());
                            }
                        }
                        None => warn!("Ignoring invalid metadata format (expected key=value): {}", kv),
                    }
                }
            }
            watcher_config.metadata = metadata;

            if let Some(patterns) = args.ignore_patterns {
                watcher_config.ignore_patterns = patterns;
            }

            let mut watcher = ClipboardWatcher::new(watcher_config, MarkdownExtractor::new());

            if let Err(e) = watcher.run().await {
                error!("Clipboard watcher terminated with error: {}", e);
            }

            info!("Clipboard Watcher has stopped.");
        }
    }
}
