# Performance Comparison

The SIEM Ensemble is designed for scenarios where traditional SIEM platforms reach their architectural limits. While Splunk and Elastic provide powerful feature sets and ecosystem integrations, their heavy reliance on general-purpose runtimes (JVM) and high-level abstractions leads to significant ingestion bottlenecks.

## Performance Metrics at a Glance

| Metric | Traditional SIEM (JVM-based) | SIEM Ensemble |
| :--- | :--- | :--- |
| **Ingestion Pipeline** | Multi-process/Multi-thread + Context Switches | Lock-free Atomic Bus (Zero-Copy) |
| **Memory** | Garbage Collected / High Allocation Rate | Fixed-memory / Zero-Copy |
| **Parsing** | Regex-heavy / High-Level Logic | Assembly-Optimized Scalar |
| **Scaling** | Horizontal Cluster Required | Vertical Single-Node (Near Line-Rate) |
| **Latency/Event** | Milliseconds | Nanoseconds |

## Why the Difference?

### 1. The Cost of Abstraction
Splunk and Elastic use high-level query and processing languages that are designed for ease of use but incur a "tax" at each layer: string copying, object deserialization, and heap allocation. In the SIEM Ensemble, data is placed once into shared memory by the Rust producer and accessed directly in-place by the analytics engine. There are zero secondary allocations.

### 2. Kernel-Space Overhead
In standard distributed systems, the network and IPC stacks are managed by the kernel. Under heavy load, the overhead of triggering interrupts and managing thread scheduling across the kernel boundary consumes the majority of available CPU cycles. The Ensemble's use of atomic ring buffers maintains the entire ingestion loop in **user space**, keeping threads tightly bound to their work and avoiding the context-switching tax.

### 3. CPU Cycles per Event
Traditional SIEMs spend hundreds or thousands of cycles to parse a timestamp, evaluate regex, and route an event. By moving timestamp parsing into optimized scalar assembly and using hardware-aware memory padding, the Ensemble reduces the "cost" of ingestion to the absolute minimum allowed by the CPU architecture.

## Summary: Scaling
- **Traditional SIEM:** Requires scaling **horizontally** (adding more nodes) as ingest rates increase, which compounds cluster management complexity and operational costs.
- **Ensemble:** Scales **vertically**. Because it saturates the memory bus rather than fighting the kernel or the VM, a single Ensemble node can outperform an entire cluster of traditional SIEM instances for raw log ingestion and processing tasks.
