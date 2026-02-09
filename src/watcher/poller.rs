use std::collections::HashSet;
use std::time::Duration;

use tokio::sync::{broadcast, oneshot};

use crate::api::feeds;
use crate::client::http::RateLimitedHttp;

use super::events::WatcherEvent;
use super::filter::WatchFilter;

/// The polling loop that watches SEC RSS feeds for new filings.
pub struct Poller {
    http: RateLimitedHttp,
    filter: WatchFilter,
    poll_interval: Duration,
    tx: broadcast::Sender<WatcherEvent>,
    shutdown_rx: oneshot::Receiver<()>,
}

impl Poller {
    pub fn new(
        http: RateLimitedHttp,
        filter: WatchFilter,
        poll_interval: Duration,
        tx: broadcast::Sender<WatcherEvent>,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            http,
            filter,
            poll_interval,
            tx,
            shutdown_rx,
        }
    }

    pub async fn run(mut self) {
        let _ = self.tx.send(WatcherEvent::Started);

        // Track seen accession numbers to avoid duplicate events
        let mut seen: HashSet<String> = HashSet::new();
        // On first poll, just populate the seen set without emitting events
        let mut first_poll = true;

        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx => {
                    let _ = self.tx.send(WatcherEvent::Stopped);
                    return;
                }
                _ = tokio::time::sleep(self.poll_interval) => {
                    match feeds::fetch_recent_feed(&self.http).await {
                        Ok(entries) => {
                            for entry in entries {
                                // Get a unique key for dedup
                                let key = entry
                                    .accession_number
                                    .clone()
                                    .unwrap_or_else(|| entry.link.clone());

                                if seen.contains(&key) {
                                    continue;
                                }
                                seen.insert(key);

                                if first_poll {
                                    continue;
                                }

                                if self.filter.matches(&entry) {
                                    let _ = self.tx.send(WatcherEvent::NewFiling(entry));
                                }
                            }
                            first_poll = false;
                        }
                        Err(e) => {
                            let _ = self.tx.send(WatcherEvent::Error(e.to_string()));
                        }
                    }

                    // Limit seen set size to prevent unbounded growth
                    if seen.len() > 10_000 {
                        // Keep only the most recent entries (rough trim)
                        let to_remove: Vec<String> = seen.iter().take(5_000).cloned().collect();
                        for key in to_remove {
                            seen.remove(&key);
                        }
                    }
                }
            }
        }
    }
}
