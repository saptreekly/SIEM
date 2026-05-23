use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::signal;
use siem::{parse_log, crypto::fnv1a_hash};
use tracing::{info, warn, error, info_span};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::thread;

mod storage;
mod control;
mod gossip;
use storage::{Storage, StorageMessage};
use gossip::GossipMesh;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    let tcp_port = std::env::var("SIEM_TCP_PORT").unwrap_or_else(|_| "8080".to_string());
    
    let storage = Arc::new(Storage::new());
    let threshold = Arc::new(AtomicU64::new(100)); // Default threshold
    
    // Spawn Janitor
    let janitor_tx = storage.tx.clone();
    thread::spawn(move || storage::run_janitor(janitor_tx));

    // Spawn Control Plane
    let node_name = format!("performer-{}", std::process::id());
    let socket_path = format!("/tmp/siem_control_{}.sock", node_name);
    let control_threshold = Arc::clone(&threshold);
    let sp_clone = socket_path.clone();
    tokio::spawn(control::start_control_listener(sp_clone, control_threshold));

    // Spawn Gossip Mesh
    tokio::spawn(gossip::start_gossip(node_name, 9000, Arc::new(GossipMesh::new(10000, 10))));

    let listener = TcpListener::bind(format!("127.0.0.1:{}", tcp_port)).await?;
    info!("SIEM listening on port {}", tcp_port);

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (socket, addr) = res.unwrap();
                info!("Accepted connection from: {}", addr);
                
                let storage_tx = storage.tx.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(socket);
                    let mut lines = reader.lines();
                    let mut dedup_cache = [u32::MAX; 2048];
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _span = info_span!("ingest_log", addr = %addr).entered();
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
                let _ = std::fs::remove_file(socket_path);
                break;
            }
        }
    }

    Ok(())
}
