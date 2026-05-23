use bloomfilter::Bloom;
use tokio::net::UdpSocket;
use tracing::{info};
use std::time::Duration;
use std::sync::{Arc, RwLock};
pub struct GossipMesh {
    pub bloom: Arc<RwLock<Bloom<u32>>>,
}

impl GossipMesh {
    pub fn new(size: usize, _bits_per_item: u32) -> Self {
        GossipMesh {
            bloom: Arc::new(RwLock::new(Bloom::new_for_fp_rate(size, 0.01))),
        }
    }
}

pub async fn start_gossip(node_name: String, port: u16, mesh: Arc<GossipMesh>) {
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind UDP");
    socket.set_broadcast(true).expect("Failed to enable broadcast");

    let broadcast_addr = format!("255.255.255.255:{}", port);
    let message = format!("PERFORMER_ALIVE:{}", node_name);

    info!("Gossip node {} started, broadcasting to {}", node_name, broadcast_addr);

    loop {
        // Broadcast Bloom Filter bitset
        {
            let bitset = {
                let bloom = mesh.bloom.read().unwrap();
                bloom.bitmap().to_vec() // Clone it to release lock immediately
            };
            let _ = socket.send_to(&bitset, &broadcast_addr).await;
        }

        // Broadcast heartbeat
        let _ = socket.send_to(message.as_bytes(), &broadcast_addr).await;

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    }
