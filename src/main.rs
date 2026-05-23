use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::signal;
use siem::{parse_log, crypto::fnv1a_hash};
use tracing::{info, warn, error};
use std::sync::Arc;
use std::thread;

mod storage;
mod control;
use storage::{Storage, StorageMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let storage = Arc::new(Storage::new());

    // Spawn Janitor
    let janitor_tx = storage.tx.clone();
    thread::spawn(move || storage::run_janitor(janitor_tx));

    // Spawn Control Plane
    tokio::spawn(control::start_control_listener("/tmp/siem_control.sock"));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("SIEM listening on port 8080");

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (socket, addr) = res?;
                info!("Accepted connection from: {}", addr);
                
                let storage_tx = storage.tx.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(socket);
                    let mut lines = reader.lines();
                    let mut dedup_cache = [u32::MAX; 2048];
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        let hash = fnv1a_hash(line.as_bytes());
                        let slot_idx = (hash as usize) & 2047;
                        
                        if dedup_cache[slot_idx] == hash {
                             continue;
                        }
                        dedup_cache[slot_idx] = hash;
                        
                        if let Some(event) = parse_log(&line) {
                            if let Err(e) = storage_tx.send(StorageMessage::Insert(event)) {
                                error!("Failed to send log to queue: {}", e);
                            }
                        } else {
                            warn!("Failed to parse log line: {}", line);
                        }
                    }
                    info!("Connection closed: {}", addr);
                });
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
