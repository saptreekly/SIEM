use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct AtomicBitArray {
    buckets: Arc<[AtomicU32; 2048]>,
}

impl AtomicBitArray {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(std::array::from_fn(|_| AtomicU32::new(0))),
        }
    }

    // Returns true if the hash was already set (collision detected)
    // Returns false if the hash was newly set
    pub fn check_and_set(&self, hash: u64) -> bool {
        let bucket = (hash & 0x7FF) as usize; // 2048 buckets
        let bit = 1 << (hash & 0xF); // 16 bits per bucket
        
        let prev = self.buckets[bucket].fetch_or(bit, Ordering::Relaxed);
        (prev & bit) != 0
    }
}
