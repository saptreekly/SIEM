package siem_analytics

import "core:fmt"
import "core:time"

// Data-Oriented Layout
LogEvent :: struct {
    timestamp: i64,
    severity:  string,
    source_ip: string,
}

// Global Structure of Arrays (SOA) fixed-size ring buffer
// 2048 is a power-of-two for efficient masking
WINDOW_SIZE :: 2048
hot_window: #soa[WINDOW_SIZE]LogEvent
cursor: int = 0

// Evaluate threat correlation threshold (Branchless-friendly structure)
evaluate_brute_force_rule :: proc(target_ip: string, lookback_window: i64) {
    count := 0
    now := time.to_unix_seconds(time.now())
    
    // We iterate over the contiguous SOA arrays for SIMD performance
    for i in 0..<WINDOW_SIZE {
        // Bitwise mask logic for correlation
        // Note: String equality in Odin is branching, but the layout is cache-friendly
        if hot_window.timestamp[i] >= (now - lookback_window) && hot_window.source_ip[i] == target_ip {
            count += 1
        }
    }

    if count > 100 {
        fmt.println("!!! HIGH-PRIORITY ALERT: Brute force detected from:", target_ip)
    }
}

// O(1) insertion into ring buffer
insert_log :: proc(event: LogEvent) {
    hot_window.timestamp[cursor] = event.timestamp
    hot_window.severity[cursor]  = event.severity
    hot_window.source_ip[cursor] = event.source_ip
    
    cursor = (cursor + 1) % WINDOW_SIZE
}

main :: proc() {
    // Setup: Load logs into the ring buffer
    insert_log(LogEvent{time.to_unix_seconds(time.now()), "INFO", "192.168.1.1"})
    
    // Simulate correlation check
    evaluate_brute_force_rule("192.168.1.1", 60) // 60s lookback

    fmt.println("Correlation engine operating with zero-allocation ring buffer.")
}
