# SIEM Ensemble: High-Velocity Log Analytics

The SIEM Ensemble is a high-performance, polyglot system designed for massive-scale log ingestion and real-time analytics. It leverages a Rust-based core for raw performance, a Zig-based forwarder for zero-copy data routing via shared memory, an Odin-based analytics engine for rapid event processing, and an Elixir-based supervisor for lifecycle management and fault tolerance.

## Architecture

The system operates on a shared-memory backbone to achieve ultra-low latency between log ingestion and analytical processing.

- **Rust Core (`src/`)**: Handles TCP-based log ingestion, deduplication using an FNV-1a hash cache, and dispatching to storage.
- **Zig Forwarder (`tools/forwarder.zig`)**: Implements a high-speed buffer mechanism, routing ingested logs directly into shared memory (`/tmp/siem_shm.bin`) using a circular ring buffer layout.
- **Odin Analytics (`tools/analytics.odin`)**: A specialized, low-latency engine that consumes structured logs directly from the shared memory circular buffer for real-time analysis.
- **Elixir Supervisor (`control_plane/`)**: Orchestrates the ensemble, traps process exits, manages socket artifacts, and provides an API for remote threshold adjustments and health monitoring.

## System Components & Data Layout

Data is transmitted using a strictly typed binary wire protocol defined as `ShmFrame` in `src/shm.rs`. This ensures byte-level compatibility between Rust and Odin.

### ShmFrame (Wire Protocol)
```rust
#[repr(C)]
pub struct ShmFrame {
    pub timestamp: i64,      // 8 bytes
    pub severity: [u8; 24],  // 24 bytes
    pub source_ip: [u8; 24], // 24 bytes
    pub facility: [u8; 24],  // 24 bytes
    pub message: [u8; 24],   // 24 bytes
}
```

## Installation & Requirements

Ensure you have the following installed on your macOS system:
- Rust (Cargo)
- Zig (0.16.0+)
- Odin (2026-05+)
- Elixir/Erlang (OTP 27+)

## Operating the Ensemble

The system includes a `Makefile` for streamlined orchestration.

### Build
To compile the entire ensemble:
```bash
make build
```

### Execution
To start the ensemble in the background:
```bash
make run
```
This stores process IDs in `siem.pid`.

### Shutdown
To stop the ensemble gracefully:
```bash
make stop
```

### Performance Load Testing
To run a full-scale load test that builds the project, orchestrates the ensemble, runs the `blaster` load generator, and performs clean teardown:
```bash
make stress-test
```

## Resilience & Supervision

The Elixir supervisor automatically traps exits from the Rust core. If the performer process crashes under heavy load, the supervisor will:
1. Detect the `exit_status`.
2. Clean up stale Unix domain socket artifacts in `/tmp/`.
3. Restart the core performer instance immediately.
4. Log the crash event for diagnostic purposes.

## Developing & Extending

- **Wire Protocol**: If adding fields to `ShmFrame` in `src/shm.rs`, ensure you update the Odin analytics parser structure and size offsets accordingly.
- **Performance**: Rust `LogEvent` uses default compiler layout optimization (`#[repr(Rust)]`) to ensure maximum CPU efficiency. Use the `ShmFrame` struct when transferring data to shared memory to maintain the strict binary contract.
