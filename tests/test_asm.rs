extern "C" {
    fn _parse_timestamp_asm(input: *const u8) -> i64;
}

fn main() {
    let timestamp = "2026-05-24T12:00:00Z";
    let input = timestamp.as_bytes().as_ptr();

    unsafe {
        let epoch = _parse_timestamp_asm(input);
        println!("ISO8601: {}, Epoch: {}", timestamp, epoch);
    }
}
