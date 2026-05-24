mod control;
mod crypto;
mod gossip;
mod shm;
mod storage;

use std::collections::HashSet;
use std::env;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::io::BufReader;
use log::info;
use env_logger;

use crate::shm::ShmRingBuffer;
use crate::storage::{Storage, StorageMessage};
use crate::gossip::GossipMesh;
use crate::crypto::fnv1a_hash;
use crate::LogEvent;

#[tokio::main]
async fn main() {
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

    // Initialize Gossip Mesh
    let mesh = Arc::new(GossipMesh {
        recent_hashes: {
            let mut arr = [AtomicU32::new(0); 2048];
            for item in arr.iter_mut() {
                *item = AtomicU32::new(0);
            }
            Arc::new(arr)
        },
    });

    // Initialize O(1) Deduplication Cache
    let dedup_cache = Arc::new(Mutex::new(HashSet::new()));

    // Bind core data ingestion loop
    let listener = TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind ingestion port");
    info!("SIEM Performer started.");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Accepted ingestion connection from: {}", addr);
                let tx = storage_tx.clone();
                let mesh_clone = Arc::clone(&mesh);
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
}
