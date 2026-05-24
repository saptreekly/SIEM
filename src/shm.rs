use libc::{madvise, MADV_SEQUENTIAL};
use memmap2::MmapMut;
use siem::LogEvent;
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicU32, Ordering};

pub const SHM_PATH: &str = "/tmp/siem_shm.bin";
pub const SHM_SIZE: usize = 1024 * 1024; // 1MB
// Pad header to 64 bytes for cache line alignment
pub const HEADER_SIZE: usize = 64; 
pub const DATA_SIZE: usize = SHM_SIZE - HEADER_SIZE;

// Pad ShmFrame to 128 bytes (2 cache lines) for alignment
#[repr(C, align(64))]
pub struct ShmFrame {
    pub timestamp: i64,
    pub severity: [u8; 24],
    pub source_ip: [u8; 24],
    pub facility: [u8; 24],
    pub message: [u8; 24],
    pub _padding: [u8; 40], // 8 + 24*4 + 40 = 128 bytes
}

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

        file.set_len(SHM_SIZE as u64)
            .expect("Failed to set SHM size");

        let mmap = unsafe { MmapMut::map_mut(&file).expect("Failed to mmap") };

        unsafe {
            madvise(mmap.as_ptr() as *mut libc::c_void, SHM_SIZE, MADV_SEQUENTIAL);
        }

        ShmRingBuffer { mmap }
    }

    pub fn write_event(&mut self, event: &LogEvent) {
        let mut frame = ShmFrame {
            timestamp: event.timestamp,
            severity: [0; 24],
            source_ip: [0; 24],
            facility: [0; 24],
            message: [0; 24],
            _padding: [0; 40],
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

        let ptr = self.mmap.as_mut_ptr();
        let head_ptr = ptr as *const AtomicU32;
        let _tail_ptr = unsafe { ptr.add(4) as *const AtomicU32 };

        let head = unsafe { (*head_ptr).load(Ordering::Acquire) } as usize;
        let write_pos = if head + len > DATA_SIZE { 0 } else { head };
        
        unsafe {
            let dst_ptr = ptr.add(HEADER_SIZE + write_pos);
            std::ptr::copy_nonoverlapping(frame_slice.as_ptr(), dst_ptr, len);
            (*head_ptr).store((write_pos + len) as u32, Ordering::Release);
        }
    }
}

unsafe impl Send for ShmRingBuffer {}
unsafe impl Sync for ShmRingBuffer {}
