const std = @import("std");

const SHM_SIZE: usize = 1024 * 1024; // 1MB for example

const HEAD_OFFSET: usize = 0;
const TAIL_OFFSET: usize = 4;
const DATA_OFFSET: usize = 8;
const DATA_SIZE: usize = SHM_SIZE - DATA_OFFSET;

pub fn main() !void {
    const file = try std.fs.openFileAbsolute("/tmp/siem_shm.bin", .{ .mode = .read_write });
    defer file.close();

    const mmap = try std.posix.mmap(null, SHM_SIZE, std.posix.PROT.WRITE, std.posix.MAP.SHARED, file.handle, 0);
    defer std.posix.munmap(mmap);

    var data_buffer = mmap[DATA_OFFSET..];

    // Initialize head and tail if shared memory is new/empty
    @memcpy(mmap[HEAD_OFFSET..HEAD_OFFSET+4], &@as(u32, 0));
    @memcpy(mmap[TAIL_OFFSET..TAIL_OFFSET+4], &@as(u32, 0));

    var head: u32 = 0;
    var tail: u32 = 0;
    
    const mock_log = "<34>1 2026-05-23T16:00:00Z test_service: This is a test log\n";
    const log_len: u32 = @intCast(mock_log.len);

    while (true) {
        // Read tail from shared memory
        tail = @bitCast(*u32, &mmap[TAIL_OFFSET]).*;

        // Calculate available space
        var available_space: u32 = 0;
        if (head >= tail) {
            available_space = DATA_SIZE - (head - tail);
        } else {
            available_space = tail - head;
        }

        if (available_space > log_len + 1) { // +1 for a null terminator or separator
            if (head + log_len > DATA_SIZE) { // Wrap around
                // Write part to end, then wrap and write rest from beginning
                var first_part_len = DATA_SIZE - head;
                @memcpy(data_buffer[head .. head + first_part_len], mock_log[0 .. first_part_len]);
                head = 0;
                @memcpy(data_buffer[head .. head + (log_len - first_part_len)], mock_log[first_part_len ..]);
                head += (log_len - first_part_len);
            } else {
                @memcpy(data_buffer[head .. head + log_len], mock_log);
                head += log_len;
            }
            // Write new head to shared memory
            @memcpy(mmap[HEAD_OFFSET..HEAD_OFFSET+4], &head);
        } else {
            // Buffer full, wait a bit
            std.time.sleep(10 * std.time.ns_per_ms);
        }
    }
}
