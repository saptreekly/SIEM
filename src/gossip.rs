use tokio::net::UdpSocket;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::info;

pub struct GossipMesh {
    pub recent_hashes: Arc<[AtomicU32; 2048]>,
}

impl GossipMesh {
    pub fn new(_capacity: usize) -> Self {
        let mut hashes = Vec::with_capacity(2048);
        for _ in 0..2048 {
            hashes.push(AtomicU32::new(u32::MAX));
        }
        GossipMesh {
            recent_hashes: Arc::new(hashes.try_into().unwrap()),
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
            // Broadcast hashes
            let mut data = Vec::with_capacity(2048 * 4);
            for i in 0..2048 {
                let h = mesh_send.recent_hashes[i].load(Ordering::Relaxed);
                if h != u32::MAX {
                    data.extend_from_slice(&h.to_le_bytes());
                }
            }
            let _ = socket_send.send_to(&data, &broadcast_addr).await;

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
                // Merge received hashes
                for chunk in data.chunks(4) {
                    if chunk.len() == 4 {
                        let hash = u32::from_le_bytes(chunk.try_into().unwrap());
                        let slot = (hash as usize) & 2047;
                        mesh_recv.recent_hashes[slot].store(hash, Ordering::Relaxed);
                    }
                }
                info!("Gossip: Merged {} remote hashes", data.len() / 4);
            }
        }
    }
}
