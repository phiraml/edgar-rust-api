pub mod api;
pub mod cli;
pub mod client;
pub mod error;
pub mod models;
pub mod standardizer;
pub mod watcher;

pub use client::EdgarClient;
pub use error::{EdgarError, Result};
