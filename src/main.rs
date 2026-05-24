mod control;
mod crypto;
mod gossip;
mod shm;
mod storage;

use std::env;
use std::sync::Arc;
use tokio::sync::atomic::AtomicU64;
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();

    let control_address = env::var("CONTROL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let threshold = Arc::new(AtomicU64::new(100));
    let ingestion_endpoint = Arc::new(std::sync::Mutex::new("http://localhost:8080".to_string()));

    tokio::spawn({
        let threshold_clone = Arc::clone(&threshold);
        let ingestion_endpoint_clone = Arc::clone(&ingestion_endpoint);
        async move {
            control::start_control_listener(control_address, threshold_clone, ingestion_endpoint_clone).await;
        }
    });

    // Start other components like gossip, shm, storage, etc.
    // ...

    info!("SIEM Conductor started.");
}
