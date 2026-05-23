pub mod crypto;
pub use crypto::fnv1a_hash;

use nom::{
    bytes::complete::take_until,
    character::complete::char,
    IResult,
};
use chrono::{DateTime, Utc};

// Define a structured representation of the log
#[derive(Debug, PartialEq, Clone)]
pub struct LogEvent {
    pub timestamp: DateTime<Utc>,
    pub severity: String,
    pub source_ip: String,
    pub facility: String,
    pub message: String,
}

// Zero-copy parser using nom
pub fn parse_log(raw: &str) -> Option<LogEvent> {
    match parse_log_nom(raw) {
        Ok((_, event)) => Some(event),
        Err(_) => None,
    }
}

fn parse_log_nom(input: &str) -> IResult<&str, LogEvent> {
    // Example: <34>1 2026-05-23T16:00:00Z localhost sshd[1234]: Failed password for root
    // Placeholder parser to keep it simple while focus is on storage
    let (input, _) = take_until(":")(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = char(' ')(input)?;
    
    Ok((input, LogEvent { 
        timestamp: Utc::now(),
        severity: "INFO".to_string(),
        source_ip: "127.0.0.1".to_string(),
        facility: "syslog".to_string(),
        message: input.to_string() 
    }))
}
