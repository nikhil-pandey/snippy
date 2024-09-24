use tracing_subscriber::{fmt, EnvFilter};

pub fn initialize_logger() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("clipboard_watcher=debug,er=error"));

    let subscriber = fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global logger");
}