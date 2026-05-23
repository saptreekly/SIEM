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
        let len = std::mem::size_of::<LogEvent>();
        let data_ptr = event as *const LogEvent as *const u8;
        let data_slice = unsafe { std::slice::from_raw_parts(data_ptr, len) };

        unsafe {
            let ptr = self.mmap.as_mut_ptr();
            
            // Read current head
            let head_ptr = ptr as *mut u32;
            let head = std::ptr::read_volatile(head_ptr);
            
            // Write data
            let write_pos = head as usize;
            
            // Note: Simplification - assume struct size fits in contiguous block.
            // If wrapping is required, logic would be more complex.
            let dst_ptr = ptr.add(HEADER_SIZE + write_pos);
            std::ptr::copy_nonoverlapping(data_slice.as_ptr(), dst_ptr, len);
            
            // Update head
            let new_head = (head + len as u32) % DATA_SIZE as u32;
            std::ptr::write_volatile(head_ptr, new_head);
        }
    }
}
