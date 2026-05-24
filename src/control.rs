use tokio::net::{TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;

pub async fn start_control_listener(control_address: String, threshold: Arc<AtomicU64>, ingestion_enabled: Arc<AtomicBool>) {
    let listener = TcpListener::bind(&control_address).await.expect("Failed to bind to TCP control address");

    info!("Control plane hardened and listening on: {}", control_address);

    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                info!("Accepted control connection from: {}", addr);
                let threshold_clone = Arc::clone(&threshold);
                let enabled_clone = Arc::clone(&ingestion_enabled);
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];
                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let command = String::from_utf8_lossy(&buffer[..n]);
                                let cmd = command.trim();
                                info!("Received control command: {}", cmd);

                                if cmd.starts_with("SET_THRESHOLD ") {
                                    if let Ok(val) = cmd[14..].parse::<u64>() {
                                        threshold_clone.store(val, Ordering::Relaxed);
                                        let response = format!("ACK: THRESHOLD SET TO {}
", val);
                                        let _ = stream.write_all(response.as_bytes()).await;
                                    }
                                } else {
                                    match cmd {
                                        "PAUSE_INGESTION" => {
                                            enabled_clone.store(false, Ordering::Relaxed);
                                            let _ = stream.write_all(b"ACK: INGESTION PAUSED
").await;
                                        }
                                        "RESUME_INGESTION" => {
                                            enabled_clone.store(true, Ordering::Relaxed);
                                            let _ = stream.write_all(b"ACK: INGESTION RESUMED
").await;
                                        }
                                        "PANIC" => {
                                            let _ = stream.write_all(b"ACK: Initiating panic!
").await;
                                            panic!("Forced crash for testing!")
                                        },
                                        _ => {
                                            let response = format!("ACK: {}
", cmd);
                                            let _ = stream.write_all(response.as_bytes()).await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading from control stream: {}", e);
                                break;
                            }
                        }
                    }
                    info!("Control connection closed.");
                });
            }
            Err(e) => error!("TCP accept error: {}", e),
        }
    }
}
