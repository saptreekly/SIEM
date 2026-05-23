package siem_analytics

import "core:fmt"
import "core:mem"
import "core:time"

// Data-Oriented Layout
LogEvent :: struct {
    timestamp: i64,
    severity:  string,
    source_ip: string,
}

// Global Structure of Arrays (SOA) dynamic slice
hot_window: #soa[dynamic]LogEvent

// Evaluate threat correlation threshold
evaluate_brute_force_rule :: proc(target_ip: string, lookback_window: i64) {
    count := 0
    now := time.to_unix_seconds(time.now())
    
    // Iterating over indices to check components contiguously
    for i in 0..<len(hot_window) {
        if hot_window.timestamp[i] >= (now - lookback_window) && hot_window.source_ip[i] == target_ip {
            count += 1
        }
    }

    if count > 100 {
        fmt.println("!!! HIGH-PRIORITY ALERT: Brute force detected from:", target_ip)
    }
}

main :: proc() {
    // Tracking Allocator for memory hygiene
    tracking_allocator: mem.Tracking_Allocator
    mem.tracking_allocator_init(&tracking_allocator, context.allocator)
    defer mem.tracking_allocator_destroy(&tracking_allocator)
    
    context.allocator = mem.tracking_allocator(&tracking_allocator)

    // Setup: Simulate loading logs into the hot window
    append(&hot_window, LogEvent{time.to_unix_seconds(time.now()), "INFO", "192.168.1.1"})
    
    // Simulate correlation check
    evaluate_brute_force_rule("192.168.1.1", 60) // 60s lookback

    // Cleanup
    delete(hot_window)

    // Check memory leaks
    if len(tracking_allocator.allocation_map) > 0 {
        fmt.println("Leaked memory:", len(tracking_allocator.allocation_map), "allocations")
    } else {
        fmt.println("Memory hygiene: Clean")
    }
}
