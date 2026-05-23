#![no_main]
use libfuzzer_sys::fuzz_target;
use siem::parse_log;

fuzz_target!(|data: &[u8]| {
    // Convert raw fuzzing bytes to &str (ignoring invalid UTF-8)
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_log(s);
    }
});
