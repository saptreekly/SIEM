# SIEM: Lightweight, High-Velocity Log Management

![Rust](https://img.shields.io/badge/language-Rust-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Status](https://img.shields.io/badge/status-Alpha-orange.svg)

## Why this SIEM?

I am building this custom, hyper-optimized SIEM because I am deeply frustrated with the bloat, complexity, and exorbitant costs of enterprise solutions like Elastic and Splunk. 

I needed a system that is:
*   **Lean:** Minimal resource footprint.
*   **Fast:** Zero-copy parsing and high-throughput ingestion.
*   **Maintainable:** Simple architecture with automated data lifecycle management (Hot/Warm/Cold tiering).
*   **Self-Contained:** No massive distributed cluster required for basic operations.

This project is my attempt to reclaim simplicity and performance in log management.

## Architecture Highlights

*   **Ingestion Pipeline:** Asynchronous TCP ingestion decoupled by an MPSC channel for maximum ingestion velocity.
*   **Optimized Parser:** Zero-copy log parsing utilizing `nom` for extreme efficiency.
*   **Automated Storage Tiering:** 
    *   **Hot Tier:** WAL-indexed SQLite for fast, concurrent writes.
    *   **Warm Tier:** Optimized SQLite for historical query.
    *   **Cold Tier:** Automated export to compressed storage.
*   **Janitor:** Background maintenance tasks that handle data migration and optimization without blocking ingestion.

## License

This project is licensed under the [MIT License](LICENSE).
