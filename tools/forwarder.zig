const std = @import("std");

const SHM_SIZE: usize = 1024 * 1024; // 1MB

const HEAD_OFFSET: usize = 0;
const TAIL_OFFSET: usize = 4;
const DATA_OFFSET: usize = 8;
const DATA_SIZE: usize = SHM_SIZE - DATA_OFFSET;

const LogEvent = extern struct {
    timestamp: i64,
    severity: [24]u8,
    source_ip: [24]u8,
    facility: [24]u8,
    message: [24]u8,
};

// Stable POSIX C FFI to shield the application from Zig standard library namespace changes
extern "c" fn open(path: [*:0]const u8, flags: c_int, mode: c_uint) c_int;
extern "c" fn mmap(addr: ?*anyopaque, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: i64) ?*anyopaque;
extern "c" fn munmap(addr: *anyopaque, len: usize) c_int;
extern "c" fn close(fd: c_int) c_int;
extern "c" fn usleep(useconds: c_uint) c_int;
extern "c" fn time(t: *i64) i64;
extern "c" fn sem_open(name: [*:0]const u8, oflag: c_int, mode: c_uint, value: c_uint) ?*anyopaque;
extern "c" fn sem_wait(sem: ?*anyopaque) c_int;
extern "c" fn sem_post(sem: ?*anyopaque) c_int;
extern "c" fn sem_close(sem: ?*anyopaque) c_int;

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

    const sem = sem_open("/siem_shm_sem", O_CREAT, 0o666, 1);
    if (sem == null or sem == @as(?*anyopaque, @ptrFromInt(0xFFFFFFFFFFFFFFFF))) return error.SemOpenFailed;
    defer _ = sem_close(sem);

    // Remove the blind reset of head/tail pointers
    // @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + HEAD_OFFSET))).* = 0;
    // @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + TAIL_OFFSET))).* = 0;

    var head: u32 = 0;
    var tail: u32 = 0;

    while (true) {
        _ = sem_wait(sem);
        tail = @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + TAIL_OFFSET))).*;
        head = @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + HEAD_OFFSET))).*;

        const event_size = @sizeOf(LogEvent);
        var head_to_write = head;
        var can_fit = false;

        // Check if it fits without wrapping
        if (head + event_size <= DATA_SIZE) {
            if (head >= tail) {
                if ((DATA_SIZE - head) + tail >= event_size) can_fit = true;
            } else {
                if (tail - head >= event_size) can_fit = true;
            }
        } else {
            // Must wrap, check if it fits at 0
            if (tail >= event_size) {
                head_to_write = 0;
                can_fit = true;
            }
        }

        if (can_fit) {
            // Construct mock event
            var t: i64 = 0;
            var event = std.mem.zeroInit(LogEvent, .{
                .timestamp = time(&t),
                .severity = std.mem.zeroes([24]u8),
                .source_ip = std.mem.zeroes([24]u8),
                .facility = std.mem.zeroes([24]u8),
                .message = std.mem.zeroes([24]u8),
            });
            
            // Fill fields
            const severity = "INFO";
            const source_ip = "127.0.0.1";
            const facility = "auth";
            const message = "test log";
            
            std.mem.copyForwards(u8, &event.severity, severity);
            std.mem.copyForwards(u8, &event.source_ip, source_ip);
            std.mem.copyForwards(u8, &event.facility, facility);
            std.mem.copyForwards(u8, &event.message, message);
            
            // Copy struct bytes
            const event_bytes: [*]const u8 = @ptrCast(&event);
            @memcpy(data_buffer[head_to_write .. head_to_write + event_size], event_bytes[0..event_size]);
            
            head = head_to_write + @as(u32, @intCast(event_size));
            @as(*volatile u32, @alignCast(@ptrCast(mmap_slice + HEAD_OFFSET))).* = head;
            _ = sem_post(sem);
        } else {
            _ = sem_post(sem);
            _ = usleep(10000); // 10ms
        }
    }
}
