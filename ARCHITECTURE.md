# SIEM Ensemble Architecture

The SIEM Ensemble is designed for **extreme velocity**. Traditional SIEM architectures (like Splunk or Elastic) rely on generalized frameworks (JVM, large-scale distributed message queues, heavy process-isolation) that introduce significant overhead through kernel context switching, memory allocation/garbage collection, and data serialization.

The SIEM Ensemble eliminates these overheads by treating the system as a **tightly-coupled memory bus**.

## Core Design Principles

### 1. Lock-Free Synchronization
Instead of relying on OS-level semaphores or mutexes—which trigger kernel-mode transitions and thread parking/unparking—the Ensemble uses an **Atomic Ring Buffer**. Producer threads (Rust) and Consumer threads (Odin) coordinate via lock-free atomic `head` and `tail` pointers (`AtomicU32`) residing in shared memory. This allows threads to synchronize using CPU-level memory barriers, maintaining execution within user space.

### 2. Zero-Copy Shared Memory Bus
Events are never serialized/deserialized or re-allocated once they enter the ingestion loop. The Rust producer writes directly into a pre-allocated shared memory region (`ShmRingBuffer`). The Odin analytics engine then consumes the raw `ShmFrame` structures directly from that same memory location. This zero-copy approach eliminates the most expensive operations in high-scale ingestion: memory pressure and GC pauses.

### 3. Assembly-Optimized Hot Paths
The ingestion path is optimized at the assembly level. Timestamp parsing—the most frequent operation—is performed using highly tuned, scalar x86_64/AArch64 assembly that bypasses standard library overhead and branchy high-level logic. By using specific instructions for digit conversion and leap-year calculations, we achieve a cycles-per-event count that is nearly impossible to match in higher-level languages.

### 4. Hardware-Aware Locality
- **Cache-Line Alignment:** Every `ShmFrame` is padded to 128 bytes (2x 64-byte cache lines) and aligned in memory. This prevents "false sharing," a phenomenon where multiple cores fight over the same cache line, effectively enabling parallel producer-consumer execution without stalling.
- **Sequential Hinting:** Through `madvise(MADV_SEQUENTIAL)`, the Ensemble signals the Linux kernel to optimize its page-fault and prefetching behavior for our specific high-velocity, append-only memory usage pattern.
