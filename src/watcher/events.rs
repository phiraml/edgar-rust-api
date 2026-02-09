use crate::models::feed::FeedEntry;

/// Events emitted by the filing watcher.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// A new filing was detected.
    NewFiling(FeedEntry),
    /// The watcher encountered an error but will continue.
    Error(String),
    /// The watcher has started.
    Started,
    /// The watcher has stopped.
    Stopped,
}

/// Trait for handling watcher events.
pub trait EventHandler: Send + Sync + 'static {
    fn handle(&self, event: WatcherEvent);
}

/// Simple event handler that prints to stdout.
pub struct PrintHandler;

impl EventHandler for PrintHandler {
    fn handle(&self, event: WatcherEvent) {
        match event {
            WatcherEvent::NewFiling(entry) => {
                println!(
                    "[NEW FILING] {} - {} ({})",
                    entry.company_name.as_deref().unwrap_or("Unknown"),
                    entry.form_type.as_deref().unwrap_or("Unknown"),
                    entry.title
                );
            }
            WatcherEvent::Error(err) => {
                eprintln!("[WATCHER ERROR] {}", err);
            }
            WatcherEvent::Started => {
                println!("[WATCHER] Started watching for new filings...");
            }
            WatcherEvent::Stopped => {
                println!("[WATCHER] Stopped.");
            }
        }
    }
}
