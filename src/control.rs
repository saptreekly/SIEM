use tokio::net::{UnixListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub async fn start_control_listener(socket_path: String, threshold: Arc<AtomicU64>) {
    // Clean up existing socket file if it exists
    let _ = fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind to UDS");
    info!("Control plane listening on: {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let threshold_clone = Arc::clone(&threshold);
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
