use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::signal;
use siem::{parse_log, crypto::fnv1a_hash};
use tracing::{info, warn, error, info_span};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

mod storage;
mod control;
mod gossip;
mod shm;
use storage::{Storage, StorageMessage};
use gossip::GossipMesh;
use shm::ShmRingBuffer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    let threshold = Arc::new(AtomicU64::new(100)); // Default threshold
    let is_ingestion_enabled = Arc::new(AtomicBool::new(true));

    // Initialize SHM and pass to Storage
    info!("Initializing SHM...");
    let shm = ShmRingBuffer::new();
    let storage = Arc::new(Storage::new(Some(shm)));

    // Spawn Janitor
    let janitor_tx = storage.tx.clone();
    thread::spawn(move || storage::run_janitor(janitor_tx));

    // Spawn Control Plane
    let node_name = format!("performer_{}", std::process::id());
    let socket_path = format!("/tmp/siem_control_{}.sock", node_name);
    let control_threshold = Arc::clone(&threshold);
    let control_enabled = Arc::clone(&is_ingestion_enabled);
    let sp_clone = socket_path.clone();
    tokio::spawn(control::start_control_listener(sp_clone, control_threshold, control_enabled));

    // Spawn Gossip Mesh
    let gossip_mesh = Arc::new(GossipMesh::new(10000));
    let mesh_clone = Arc::clone(&gossip_mesh);
    tokio::spawn(gossip::start_gossip(node_name, 9000, mesh_clone));

    let enabled = Arc::clone(&is_ingestion_enabled);
    thread::spawn(move || {
        loop {
            if !enabled.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("SIEM listening on port 8080");

    let mut terminate_signal = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (socket, addr) = res.unwrap();
                info!("Accepted connection from: {}", addr);
                
                let storage_tx = storage.tx.clone();
                let mesh = Arc::clone(&gossip_mesh);
                let ingestion_enabled = Arc::clone(&is_ingestion_enabled);
                
                tokio::spawn(async move {
                    let reader = BufReader::new(socket);
                    let mut lines = reader.lines();
                    let mut dedup_cache = [u32::MAX; 2048];
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        // Check if ingestion is paused
                        if !ingestion_enabled.load(Ordering::Relaxed) {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }

                        let _span = info_span!("ingest_log", addr = %addr).entered();
                        let hash = fnv1a_hash(line.as_bytes());
                        let slot_idx = (hash as usize) & 2047;
                        
                        // Check local O(1) cache
                        if dedup_cache[slot_idx] == hash {
                             continue;
                        }
                        
                        // Check global Gossip hashes
                        if mesh.recent_hashes[slot_idx].load(Ordering::Relaxed) == hash {
                             continue;
                        }
                        
                        dedup_cache[slot_idx] = hash;
                        mesh.recent_hashes[slot_idx].store(hash, Ordering::Relaxed);
                        
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
                let _ = storage.tx.send(StorageMessage::Shutdown);
                let _ = std::fs::remove_file(socket_path);
                break;
            }
            _ = terminate_signal.recv() => {
                info!("SIGTERM received, shutting down...");
                let _ = storage.tx.send(StorageMessage::Shutdown);
                let _ = std::fs::remove_file(socket_path);
                break;
            }
        }
    }

    Ok(())
}
