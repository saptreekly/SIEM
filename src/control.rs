use tokio::net::{UnixListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use std::fs;

pub async fn start_control_listener(socket_path: &str) {
    // Clean up existing socket file if it exists
    let _ = fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path).expect("Failed to bind to UDS");
    info!("Control plane listening on: {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];
                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => break, // Connection closed
                            Ok(n) => {
                                let command = String::from_utf8_lossy(&buffer[..n]);
                                let cmd = command.trim();
                                info!("Received control command: {}", cmd);

                                match cmd {
                                    "PANIC" => panic!("Forced crash for testing!"),
                                    _ => {
                                        let response = format!("ACK: {}
", cmd);
                                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                                            error!("Failed to write to UDS: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to read from UDS: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => error!("UDS accept error: {}", e),
        }
    }
}
