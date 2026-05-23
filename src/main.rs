use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::signal;
use siem::{parse_log, crypto::fnv1a_hash};
use tracing::{info, warn, error};
use std::sync::Arc;

mod storage;
mod control;
use storage::{Storage, StorageMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let storage = Arc::new(Storage::new());

    // Spawn Janitor
    let janitor_tx = storage.tx.clone();
    tokio::spawn(storage::run_janitor(janitor_tx));

    // Spawn Control Plane
    tokio::spawn(control::start_control_listener("/tmp/siem_control.sock"));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("SIEM listening on port 8080");

    loop {
        tokio::select! {
            res = listener.accept() => {
// ... (rest of main)
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received, shutting down...");
                let _ = std::fs::remove_file("/tmp/siem_control.sock");
                break;
            }
        }
    }

    Ok(())
}

