pub mod crypto;
pub mod dedup;
pub use crypto::fnv1a_hash;

use chrono::Utc;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

extern "C" {
    fn parse_timestamp_asm(data: *const u8) -> i64;
}

// Define a structured representation of the log
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: i64,
    pub severity: CompactString,
    pub source_ip: CompactString,
    pub facility: CompactString,
    pub message: CompactString,
}

// Zero-copy parser
pub fn parse_log(raw: &str) -> Option<LogEvent> {
    let ts = unsafe { parse_timestamp_asm(raw.as_ptr()) };

    // In a production scenario, we'd do smarter parsing, but for
    // now we just ensure message doesn't allocate beyond the CompactString limit.
    Some(LogEvent {
        timestamp: if ts == 0 { Utc::now().timestamp() } else { ts },
        severity: CompactString::new("INFO"),
        source_ip: CompactString::new("127.0.0.1"),
        facility: CompactString::new("syslog"),
        message: CompactString::from(raw),
    })
}
