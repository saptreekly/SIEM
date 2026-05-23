use memmap2::MmapMut;
use std::fs::{OpenOptions};

pub const SHM_PATH: &str = "/tmp/siem_shm.bin";
pub const SHM_SIZE: usize = 1024 * 1024; // 1MB

pub struct ShmRingBuffer {
    _mmap: MmapMut,
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
        
        ShmRingBuffer { _mmap: mmap }
    }
}
