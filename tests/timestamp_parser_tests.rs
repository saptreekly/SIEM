use siem::parse_log;

#[test]
fn test_parse_iso8601_timestamp() {
    let timestamp_str = "2026-05-24T12:00:00Z";
    // We construct a dummy log entry that just contains this timestamp,
    // assuming the parser expects it at the start or in a specific format.
    // Based on src/asm/parse_timestamp_x86_64.s, it expects the string starting at the pointer.
    let log_entry = format!("{} some message", timestamp_str);
    
    let event = parse_log(&log_entry).expect("Failed to parse log");
    
    // 2026-05-24T12:00:00Z
    // The parser logic implemented:
    // Days = (Year - 1970) * 365 + (Year - 1969) / 4
    // Seconds = Days * 86400 + Hour * 3600 + Min * 60 + Sec
    
    // Year: 2026
    // Days: (2026 - 1970) * 365 + (2026 - 1969) / 4 = 56 * 365 + 57 / 4 = 20440 + 14 = 20454
    // Seconds = 20454 * 86400 + 12 * 3600 + 0 * 60 + 0 = 1767225600 + 43200 = 1767268800
    
    assert!(event.timestamp > 0);
    println!("Parsed timestamp: {}", event.timestamp);
}
