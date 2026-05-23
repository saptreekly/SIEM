const std = @import("std");

const SHM_SIZE: usize = 1024 * 1024; // 1MB

const HEAD_OFFSET: usize = 0;
const TAIL_OFFSET: usize = 4;
const DATA_OFFSET: usize = 8;
const DATA_SIZE: usize = SHM_SIZE - DATA_OFFSET;

// C FFI for macOS
extern "c" fn open(path: [*:0]const u8, flags: c_int, mode: c_uint) c_int;
extern "c" fn mmap(addr: ?*anyopaque, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: i64) ?*anyopaque;
extern "c" fn munmap(addr: *anyopaque, len: usize) c_int;
extern "c" fn close(fd: c_int) c_int;

const O_RDWR = 0x0002;
const O_CREAT = 0x0200;
const PROT_WRITE = 0x02;
const MAP_SHARED = 0x0001;

pub fn main() !void {
    const fd = open("/tmp/siem_shm.bin", O_RDWR | O_CREAT, 0o666);
    if (fd == -1) return error.OpenFileFailed;
    defer _ = close(fd);

    const mmap_ptr = mmap(null, SHM_SIZE, PROT_WRITE, MAP_SHARED, fd, 0);
    if (mmap_ptr == null or mmap_ptr == @as(?*anyopaque, @ptrFromInt(0xFFFFFFFFFFFFFFFF))) return error.MmapFailed;
    defer _ = munmap(mmap_ptr.?, SHM_SIZE);

    const mmap_slice: [*]u8 = @ptrCast(mmap_ptr);
    var data_buffer = mmap_slice[DATA_OFFSET..SHM_SIZE];

    // Initialize head and tail
    @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + HEAD_OFFSET))).* = 0;
    @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + TAIL_OFFSET))).* = 0;

    var head: u32 = 0;
    var tail: u32 = 0;

    const mock_log = "<34>1 2026-05-23T16:00:00Z test_service: This is a test log\n";
    const log_len: u32 = @intCast(mock_log.len);

    while (true) {
        tail = @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + TAIL_OFFSET))).*;

        var available_space: u32 = 0;
        if (head >= tail) {
            available_space = @as(u32, @intCast(DATA_SIZE)) - (head - tail);
        } else {
            available_space = tail - head;
        }

        if (available_space > log_len + 1) {
            if (head + log_len > DATA_SIZE) {
                const first_part_len: u32 = @intCast(DATA_SIZE - head);
                @memcpy(data_buffer[head .. head + first_part_len], mock_log[0 .. first_part_len]);
                head = 0;
                @memcpy(data_buffer[head .. head + (log_len - first_part_len)], mock_log[first_part_len ..]);
                head += (log_len - first_part_len);
            } else {
                @memcpy(data_buffer[head .. head + log_len], mock_log);
                head += log_len;
            }
            @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + HEAD_OFFSET))).* = head;
        } else {
            std.time.sleep(10 * std.time.ns_per_ms);
        }
    }
}
