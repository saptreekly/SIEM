//! High-performance string hashing.
//!
//! Provides an assembly-optimized FNV-1a 32-bit hashing implementation.

extern "C" {
    fn fnv1a_hash_asm(data: *const u8, len: usize) -> u32;
}

/// Computes the 32-bit FNV-1a hash of a byte slice.
#[inline(always)]
pub fn fnv1a_hash(data: &[u8]) -> u32 {
    unsafe {
        fnv1a_hash_asm(data.as_ptr(), data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_consistency() {
        let log = b"Failed password for root";
        let hash1 = fnv1a_hash(log);
        let hash2 = fnv1a_hash(log);
        assert_eq!(hash1, hash2, "Hashing identical strings should produce identical results");
    }

    #[test]
    fn test_fnv1a_values() {
        // Known FNV-1a 32-bit test vectors
        assert_eq!(fnv1a_hash(b""), 0x811c9dc5);
        assert_eq!(fnv1a_hash(b"a"), 0x050c5d1f);
    }
}
