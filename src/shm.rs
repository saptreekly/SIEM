use memmap2::MmapMut;
use std::fs::{OpenOptions};
use siem::LogEvent;

pub const SHM_PATH: &str = "/tmp/siem_shm.bin";
pub const SHM_SIZE: usize = 1024 * 1024; // 1MB
pub const HEADER_SIZE: usize = 8; // 4 byte head, 4 byte tail
pub const DATA_SIZE: usize = SHM_SIZE - HEADER_SIZE;

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
        let data = format!("{:?}", event).into_bytes();
        let len = data.len();
        if len + 4 > DATA_SIZE { return; } // Simplified: check if fits at all

        unsafe {
            let ptr = self.mmap.as_mut_ptr();
            
            // Read current head
            let head_ptr = ptr as *mut u32;
            let head = std::ptr::read_volatile(head_ptr);
            
            // Write data with wrapping
            let write_pos = head as usize;
            for i in 0..len {
                let pos = (write_pos + i) % DATA_SIZE;
                std::ptr::write_volatile(ptr.add(HEADER_SIZE + pos), data[i]);
            }
            
            // Update head
            let new_head = (head + len as u32) % DATA_SIZE as u32;
            std::ptr::write_volatile(head_ptr, new_head);
        }
    }
}
