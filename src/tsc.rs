#[inline(always)]
pub fn read_tsc() -> u64 {
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
