use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::signal;
use siem::{parse_log, crypto::fnv1a_hash};
use tracing::{info, warn, error};
use std::sync::Arc;

mod storage;
use storage::{Storage, StorageMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let storage = Arc::new(Storage::new());
    
    // Spawn Janitor
    let janitor_tx = storage.tx.clone();
    tokio::spawn(storage::run_janitor(janitor_tx));

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
                    let mut dedup_cache = [u32::MAX; 1000];
                    let mut cache_idx = 0;
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        let hash = fnv1a_hash(line.as_bytes());
                        if dedup_cache.contains(&hash) {
                            continue; // Drop duplicate
                        }
                        dedup_cache[cache_idx] = hash;
                        cache_idx = (cache_idx + 1) % 1000;
                        
                        if let Some(event) = parse_log(&line) {
                            if let Err(e) = storage_tx.send(StorageMessage::Insert(event)).await {
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
                break;
            }
        }
    }

    Ok(())
}
