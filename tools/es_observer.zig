const std = @import("std");

// Fixed-size binary protocol struct for SHM
const KernelExecEvent = extern struct {
    target_pid: u32,
    timestamp: i64,
    path_len: u32,
    path: [256]u8,
};

const SHM_HEADER_SIZE = 8;
const SHM_SIZE = 1024 * 1024;

pub fn main() !void {
    const file = try std.fs.openFileAbsolute("/tmp/siem_shm.bin", .{ .mode = .read_write });
    defer file.close();
    
    const mmap = try std.posix.mmap(null, SHM_SIZE, std.posix.PROT.WRITE, std.posix.MAP.SHARED, file.handle, 0);
    defer std.posix.munmap(mmap);
    
    // Simulate event ingestion
    var event = KernelExecEvent{
        .target_pid = 1234,
        .timestamp = std.time.timestamp(),
        .path_len = 11,
        .path = [_]u8{0} ** 256,
    };
    @memcpy(event.path[0..11], "/usr/bin/ls");

    // Write to SHM (Ring Buffer Logic)
    const write_head = std.mem.readInt(u32, mmap[0..4], .little);
    const offset = SHM_HEADER_SIZE + write_head;
    
    // Serialize and write
    const event_bytes = std.mem.asBytes(&event);
    @memcpy(mmap[offset..offset + event_bytes.len], event_bytes);
    
    // Update head
    const new_head = (write_head + event_bytes.len) % (SHM_SIZE - SHM_HEADER_SIZE);
    std.mem.writeInt(u32, mmap[0..4], new_head, .little);
}
