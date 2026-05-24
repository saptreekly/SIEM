use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU64;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use log::info;

pub async fn start_control_listener(control_address: String, threshold: Arc<AtomicU64>, ingestion_endpoint: Arc<Mutex<String>>) {
    let listener = TcpListener::bind(&control_address).await.expect("Failed to bind to TCP control address");

    info!("Control plane hardened and listening on: {}", control_address);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Accepted control connection from: {}", addr);
                let threshold_clone = Arc::clone(&threshold);
                let ingestion_endpoint_clone = Arc::clone(&ingestion_endpoint);
                tokio::spawn(async move {
                    let stream = stream;
                    handle_control_connection(stream, threshold_clone, ingestion_endpoint_clone).await;
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

async fn handle_control_connection(mut stream: TcpStream, threshold: Arc<AtomicU64>, ingestion_endpoint: Arc<Mutex<String>>) {
    // Handle incoming commands
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(n) if n == 0 => break, // Connection closed
            Ok(n) => {
                let command = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                match command.split_whitespace().next() {
                    Some("SET_THRESHOLD") => {
                        if let Some(value) = command.split_whitespace().nth(1) {
                            if let Ok(new_threshold) = value.parse::<u64>() {
                                threshold.store(new_threshold, std::sync::atomic::Ordering::SeqCst);
                                stream.write_all(b"Threshold updated\n").await.unwrap();
                            } else {
                                stream.write_all(b"Invalid threshold value\n").await.unwrap();
                            }
                        } else {
                            stream.write_all(b"Missing threshold value\n").await.unwrap();
                        }
                    }
                    Some("INGESTION_ENDPOINT") => {
                        if let Some(endpoint) = command.split_whitespace().nth(1) {
                            {
                                let mut endpoint_lock = ingestion_endpoint.lock().unwrap();
                                *endpoint_lock = endpoint.to_string();
                            }
                            stream.write_all(b"Ingestion endpoint updated\n").await.unwrap();
                        } else {
                            stream.write_all(b"Missing ingestion endpoint\n").await.unwrap();
                        }
                    }
                    _ => {
                        stream.write_all(b"Unknown command\n").await.unwrap();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from stream: {}", e);
                break;
            }
        }
    }
}
