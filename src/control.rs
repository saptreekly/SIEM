use tokio::net::{UnixListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use std::fs;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::os::unix::fs::PermissionsExt;

pub async fn start_control_listener(socket_path: String, threshold: Arc<AtomicU64>, ingestion_enabled: Arc<AtomicBool>) {
    // Clean up existing socket file if it exists
    let _ = fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind to UDS");
    
    // Hardening: Restrict socket file permissions to owner-only (600)
    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&socket_path, permissions).expect("Failed to set UDS permissions");

    info!("Control plane hardened and listening on: {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
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
                                        "PANIC" => panic!("Forced crash for testing!"),
                                        _ => {
                                            let response = format!("ACK: {}
", cmd);
                                            let _ = stream.write_all(response.as_bytes()).await;
                                        }
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Err(e) => error!("UDS accept error: {}", e),
        }
    }
}
