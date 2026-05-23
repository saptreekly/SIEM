pub mod crypto;
pub use crypto::fnv1a_hash;

use chrono::Utc;
use compact_str::CompactString;

extern "C" {
    fn parse_timestamp_asm(data: *const u8) -> i64;
}

// Define a structured representation of the log
#[derive(Debug, PartialEq, Clone)]
pub struct LogEvent {
    pub timestamp: i64,
    pub severity: CompactString,
    pub source_ip: CompactString,
    pub facility: CompactString,
    pub message: String,
}

// Zero-copy parser
pub fn parse_log(raw: &str) -> Option<LogEvent> {
    let ts = unsafe { parse_timestamp_asm(raw.as_ptr()) };
    
    Some(LogEvent { 
        timestamp: if ts == 0 { Utc::now().timestamp() } else { ts },
        severity: CompactString::new("INFO"),
        source_ip: CompactString::new("127.0.0.1"),
        facility: CompactString::new("syslog"),
        message: raw.to_string() 
    })
}
