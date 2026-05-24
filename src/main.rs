mod control;
mod crypto;
mod gossip;
mod shm;
mod storage;

use std::collections::HashSet;
use std::env;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::signal;
use tokio::signal::unix::SignalKind;
use log::info;
use env_logger;

use crate::shm::ShmRingBuffer;
use crate::storage::{Storage, StorageMessage};
use crate::gossip::GossipMesh;
use crate::crypto::fnv1a_hash;
use siem::LogEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 1. Runtime Infrastructure & Control Tasks
    let control_address = env::var("CONTROL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let threshold = Arc::new(AtomicU64::new(100));
    let ingestion_endpoint = Arc::new(Mutex::new("http://localhost:8080".to_string()));

    tokio::spawn({
        let threshold_clone = Arc::clone(&threshold);
        let ingestion_endpoint_clone = Arc::clone(&ingestion_endpoint);
        async move {
            control::start_control_listener(control_address, threshold_clone, ingestion_endpoint_clone).await;
        }
    });

    // 2. Re-implement the High-Velocity Ingestion Loop
    let shm = ShmRingBuffer::new();
    let storage = Storage::new(Some(shm));
    let storage_tx = storage.tx.clone();

    // Re-Spawn the Maintenance Janitor Thread
    let janitor_tx = storage_tx.clone();
    std::thread::spawn(move || storage::run_janitor(janitor_tx));

    // Re-Integrate the P2P Gossip Mesh Engine
    let gossip_mesh = Arc::new(GossipMesh::new(10000));
    let mesh_clone = Arc::clone(&gossip_mesh);
    let node_name = "siem-node-1".to_string();
    tokio::spawn(gossip::start_gossip(node_name, 9000, mesh_clone));

    // Initialize O(1) Deduplication Cache
    let dedup_cache = Arc::new(Mutex::new(HashSet::new()));

    // Bind core data ingestion loop
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("SIEM Performer started.");

    let (terminate_tx, mut terminate_signal) = tokio_mpsc::channel(1);
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
        sigterm.recv().await;
        let _ = terminate_tx.send(()).await;
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
            let _ = storage_tx.send(StorageMessage::Shutdown);
        }
        _ = terminate_signal.recv() => {
            info!("Received terminate signal");
            let _ = storage_tx.send(StorageMessage::Shutdown);
        }
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("Accepted ingestion connection from: {}", addr);
                        let tx = storage_tx.clone();
                        let mesh_clone = Arc::clone(&gossip_mesh);
                        let dedup_clone = Arc::clone(&dedup_cache);

                        tokio::spawn(async move {
                            let mut reader = BufReader::new(stream);
                            let mut line = String::new();

                            loop {
                                line.clear();
                                match reader.read_line(&mut line).await {
                                    Ok(0) => break,
                                    Ok(_) => {
                                        let log_line = line.trim().to_string();
                                        if log_line.is_empty() {
                                            continue;
                                        }

                                        let hash = fnv1a_hash(log_line.as_bytes());
                                        let bucket = (hash & 0x7FF) as usize; // 2048 buckets

                                        // O(1) local dedup check
                                        {
                                            let mut cache = dedup_clone.lock().unwrap();
                                            if cache.contains(&hash) {
                                                continue;
                                            }
                                            cache.insert(hash);
                                        }

                                        // Evaluate gossip mesh check alongside local direct-mapped cache slot array
                                        if mesh_clone.recent_hashes[bucket].load(Ordering::Relaxed) & (1 << (hash & 0xF)) != 0 {
                                            continue;
                                        }

                                        // Update non-blocking global atomics gossip matrix
                                        mesh_clone.recent_hashes[bucket].fetch_or(1 << (hash & 0xF), Ordering::Relaxed);

                                        // Parse log event
                                        let event = LogEvent {
                                            timestamp: std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs() as i64,
                                            severity: "INFO".into(),
                                            source_ip: "127.0.0.1".into(),
                                            facility: "auth".into(),
                                            message: log_line.into(),
                                        };

                                        // Dispatch over crossbeam channel to synchronous database thread
                                        if tx.send(StorageMessage::Insert(event)).is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error reading line: {}", e);
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                    }
                }
            }
        } => {}
    }
    Ok(())
}

