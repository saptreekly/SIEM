use tokio::net::UdpSocket;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use tracing::info;

pub struct GossipMesh {
    pub recent_hashes: Arc<RwLock<Vec<u32>>>,
}

impl GossipMesh {
    pub fn new(_capacity: usize) -> Self {
        GossipMesh {
            recent_hashes: Arc::new(RwLock::new(Vec::with_capacity(100))),
        }
    }
}

pub async fn start_gossip(node_name: String, port: u16, mesh: Arc<GossipMesh>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await.expect("Failed to bind UDP");
    socket.set_broadcast(true).expect("Failed to enable broadcast");
    
    let broadcast_addr = format!("255.255.255.255:{}", port);
    let message = format!("PERFORMER_ALIVE:{}", node_name);
    
    let socket_send = Arc::new(socket);
    let socket_recv = Arc::clone(&socket_send);
    let mesh_send = Arc::clone(&mesh);
    let mesh_recv = Arc::clone(&mesh);

    // Broadcast task
    tokio::spawn(async move {
        loop {
            // Broadcast recent hashes
            {
                let data = {
                    let hashes = mesh_send.recent_hashes.read().unwrap();
                    let mut d = Vec::with_capacity(hashes.len() * 4);
                    for hash in hashes.iter() {
                        d.extend_from_slice(&hash.to_le_bytes());
                    }
                    d
                }; // lock dropped here
                let _ = socket_send.send_to(&data, &broadcast_addr).await;
            }

            // Broadcast heartbeat
            let _ = socket_send.send_to(message.as_bytes(), &broadcast_addr).await;

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Receive task
    let mut buf = [0u8; 1024];
    loop {
        if let Ok((n, _)) = socket_recv.recv_from(&mut buf).await {
            let data = &buf[..n];
            if !data.starts_with(b"PERFORMER_ALIVE:") {
                // Merge received hashes into local cache
                let mut hashes = mesh_recv.recent_hashes.write().unwrap();
                for chunk in data.chunks(4) {
                    if chunk.len() == 4 {
                        let hash = u32::from_le_bytes(chunk.try_into().unwrap());
                        if !hashes.contains(&hash) {
                            hashes.push(hash);
                            if hashes.len() > 100 { hashes.remove(0); } // Keep size
                        }
                    }
                }
                info!("Gossip: Merged {} remote hashes", data.len() / 4);
            }
        }
    }
}
