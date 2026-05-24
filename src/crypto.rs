//! High-performance string hashing.
//!
//! Provides an assembly-optimized FNV-1a 32-bit hashing implementation.

use std::sync::atomic::{AtomicU64, Ordering};

lazy_static::lazy_static! {
    static ref FNV_CYCLES: AtomicU64 = AtomicU64::new(0);
    static ref FNV_COUNT: AtomicU64 = AtomicU64::new(0);
}

#[inline(always)]
fn read_tsc() -> u64 {
    unsafe {
        let mut eax: u32;
        let mut edx: u32;
        std::arch::asm!(
            "rdtsc",
            out("eax") eax,
            out("edx") edx,
            options(nostack)
        );
        ((edx as u64) << 32) | (eax as u64)
    }
}

extern "C" {
    fn fnv1a_hash_asm(data: *const u8, len: usize) -> u32;
}

/// Computes the 32-bit FNV-1a hash of a byte slice.
pub fn fnv1a_hash(data: &[u8]) -> u32 {
    let start = read_tsc();
    let hash = unsafe { fnv1a_hash_asm(data.as_ptr(), data.len()) };
    let end = read_tsc();

    FNV_CYCLES.fetch_add(end - start, Ordering::Relaxed);
    let count = FNV_COUNT.fetch_add(1, Ordering::Relaxed);

    if count % 1000000 == 0 {
        let avg = FNV_CYCLES.load(Ordering::Relaxed) / (count + 1);
        eprintln!("[FNV1a Telemetry] Avg Cycles: {}", avg);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_values() {
        // Known FNV-1a 32-bit test vectors
        assert_eq!(fnv1a_hash(b""), 0x811c9dc5);
        assert_eq!(fnv1a_hash(b"a"), 0xe40c292c);
    }
}
