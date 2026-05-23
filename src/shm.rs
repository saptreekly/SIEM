use memmap2::MmapMut;
use std::fs::{OpenOptions};
use siem::LogEvent;

pub const SHM_PATH: &str = "/tmp/siem_shm.bin";
pub const SHM_SIZE: usize = 1024 * 1024; // 1MB

pub struct ShmRingBuffer {
    mmap: MmapMut,
}

impl ShmRingBuffer {
    pub fn new() -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(SHM_PATH)
            .expect("Failed to open SHM file");
        
        file.set_len(SHM_SIZE as u64).expect("Failed to set SHM size");
        
        let mmap = unsafe { MmapMut::map_mut(&file).expect("Failed to mmap") };
        
        ShmRingBuffer { mmap }
    }

    pub fn write_event(&mut self, event: &LogEvent) {
        // Simple ring buffer: [head:8][tail:8][data...]
        // In a real implementation, use atomic head/tail.
        // For this prototype, we'll write at a fixed offset after header.
        let data = bincode::serialize(event).expect("Failed to serialize");
        let len = data.len();
        if len + 16 > SHM_SIZE { return; } // Too large

        unsafe {
            let ptr = self.mmap.as_mut_ptr();
            // Write length then data
            std::ptr::copy_nonoverlapping(len.to_le_bytes().as_ptr(), ptr.add(16), 8);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(24), len);
        }
    }
}
