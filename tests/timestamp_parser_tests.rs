use siem::parse_log;

#[test]
fn test_parse_iso8601_timestamp_real_epoch() {
    // Case 1: The original date mentioned in the user request (2026-05-24)
    let ts_str = "2026-05-24T12:00:00Z";
    let log = format!("{} some message", ts_str);
    let event = parse_log(&log).expect("Failed to parse log");
    // Unix Epoch for 2026-05-24T12:00:00Z is 1779624000
    assert_eq!(
        event.timestamp, 1779624000,
        "Failed for 2026-05-24T12:00:00Z"
    );

    // Case 2: Before leap day in a leap year
    let ts_str_2 = "2024-02-28T00:00:00Z";
    let event_2 = parse_log(&ts_str_2).expect("Failed to parse log");
    assert_eq!(
        event_2.timestamp, 1709078400,
        "Failed for 2024-02-28T00:00:00Z"
    );

    // Case 3: On leap day
    let ts_str_3 = "2024-02-29T00:00:00Z";
    let event_3 = parse_log(&ts_str_3).expect("Failed to parse log");
    assert_eq!(
        event_3.timestamp, 1709164800,
        "Failed for 2024-02-29T00:00:00Z"
    );

    // Case 4: After leap day in a leap year
    let ts_str_4 = "2024-03-01T00:00:00Z";
    let event_4 = parse_log(&ts_str_4).expect("Failed to parse log");
    assert_eq!(
        event_4.timestamp, 1709251200,
        "Failed for 2024-03-01T00:00:00Z"
    );
}

#[test]
fn test_parse_iso8601_timestamp_boundary() {
    // 1970-01-02T00:00:00Z
    let ts_str = "1970-01-02T00:00:00Z";
    let event = parse_log(&ts_str).expect("Failed to parse log");
    assert_eq!(event.timestamp, 86400, "Failed for 1970-01-02T00:00:00Z");
}
