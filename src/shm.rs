use memmap2::MmapMut;
use std::fs::{OpenOptions};
use std::ffi::CString;
use libc::{sem_t, sem_open, sem_wait, sem_post, sem_close, O_CREAT};
use siem::LogEvent;

pub const SHM_PATH: &str = "/tmp/siem_shm.bin";
pub const SHM_SIZE: usize = 1024 * 1024; // 1MB
pub const HEADER_SIZE: usize = 8; // 4 byte head, 4 byte tail
pub const DATA_SIZE: usize = SHM_SIZE - HEADER_SIZE;
pub const SEM_NAME: &str = "/siem_shm_sem";

#[repr(C)]
pub struct ShmFrame {
    pub timestamp: i64,
    pub severity: [u8; 24],
    pub source_ip: [u8; 24],
    pub facility: [u8; 24],
    pub message: [u8; 24],
}

pub struct ShmRingBuffer {
    mmap: MmapMut,
    sem: *mut sem_t,
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
        
        let sem_name = CString::new(SEM_NAME).unwrap();
        let sem = unsafe { sem_open(sem_name.as_ptr(), O_CREAT, 0o666, 1) };
        if sem == libc::SEM_FAILED {
            panic!("Failed to open semaphore");
        }
        
        ShmRingBuffer { mmap, sem }
    }

    pub fn write_event(&mut self, event: &LogEvent) {
        let mut frame = ShmFrame {
            timestamp: event.timestamp,
            severity: [0; 24],
            source_ip: [0; 24],
            facility: [0; 24],
            message: [0; 24],
        };

        let copy_to_field = |field: &mut [u8; 24], src: &str| {
            let bytes = src.as_bytes();
            let len = std::cmp::min(bytes.len(), 24);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), field.as_mut_ptr(), len);
            }
        };

        copy_to_field(&mut frame.severity, &event.severity);
        copy_to_field(&mut frame.source_ip, &event.source_ip);
        copy_to_field(&mut frame.facility, &event.facility);
        copy_to_field(&mut frame.message, &event.message);

        let len = std::mem::size_of::<ShmFrame>();
        let frame_ptr = &frame as *const ShmFrame as *const u8;
        let frame_slice = unsafe { std::slice::from_raw_parts(frame_ptr, len) };

        unsafe {
            sem_wait(self.sem);

            let ptr = self.mmap.as_mut_ptr();
            
            // Read current head
            let head_ptr = ptr as *mut u32;
            let head = std::ptr::read_volatile(head_ptr) as usize;
            
            // Check if frame fits in remaining space
            let write_pos = if head + len > DATA_SIZE {
                0
            } else {
                head
            };
            
            // Write data
            let dst_ptr = ptr.add(HEADER_SIZE + write_pos);
            std::ptr::copy_nonoverlapping(frame_slice.as_ptr(), dst_ptr, len);
            
            // Update head
            let new_head = (write_pos + len) as u32;
            std::ptr::write_volatile(head_ptr, new_head);

            sem_post(self.sem);
        }
    }
}

impl Drop for ShmRingBuffer {
    fn drop(&mut self) {
        unsafe {
            sem_close(self.sem);
        }
    }
}

unsafe impl Send for ShmRingBuffer {}
unsafe impl Sync for ShmRingBuffer {}
