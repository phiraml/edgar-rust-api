pub mod events;
pub mod filter;
pub mod poller;

use std::time::Duration;

use tokio::sync::{broadcast, oneshot};

use crate::client::http::RateLimitedHttp;
use self::events::WatcherEvent;
use self::filter::WatchFilter;
use self::poller::Poller;

/// Configuration for the filing watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// How often to poll the SEC RSS feed (default: 60s).
    pub poll_interval: Duration,
    /// Filter for which filings to emit events for.
    pub filter: WatchFilter,
    /// Broadcast channel capacity (default: 256).
    pub channel_capacity: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(60),
            filter: WatchFilter::default(),
            channel_capacity: 256,
        }
    }
}

/// Handle returned when starting a watcher. Use this to receive events or stop the watcher.
pub struct WatcherHandle {
    /// Receive events from the watcher.
    pub rx: broadcast::Receiver<WatcherEvent>,
    /// Send to this channel to stop the watcher.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// The spawned task handle.
    task: tokio::task::JoinHandle<()>,
}

impl WatcherHandle {
    /// Stop the watcher gracefully.
    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }

    /// Subscribe to watcher events (additional receiver).
    pub fn subscribe(&self) -> broadcast::Receiver<WatcherEvent> {
        // We need the sender to subscribe, but we only have the receiver.
        // The sender is held by the poller. Users should use the rx directly
        // or create additional handles before starting.
        self.rx.resubscribe()
    }
}

/// Start watching for new SEC filings.
///
/// Returns a `WatcherHandle` that can be used to receive events and stop the watcher.
pub fn start_watcher(http: RateLimitedHttp, config: WatcherConfig) -> WatcherHandle {
    let (tx, rx) = broadcast::channel(config.channel_capacity);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let poller = Poller::new(
        http,
        config.filter,
        config.poll_interval,
        tx,
        shutdown_rx,
    );

    let task = tokio::spawn(async move {
        poller.run().await;
    });

    WatcherHandle {
        rx,
        shutdown_tx: Some(shutdown_tx),
        task,
    }
}
