use tokio::net::UdpSocket;
use tracing::{info, error};
use std::time::Duration;

pub async fn start_gossip(node_name: String, port: u16) {
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind UDP");
    socket.set_broadcast(true).expect("Failed to enable broadcast");
    
    let broadcast_addr = format!("255.255.255.255:{}", port);
    let message = format!("PERFORMER_ALIVE:{}", node_name);
    
    info!("Gossip node {} started, broadcasting to {}", node_name, broadcast_addr);

    loop {
        if let Err(e) = socket.send_to(message.as_bytes(), &broadcast_addr).await {
            error!("Gossip broadcast error: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
