const std = @import("std");

const SHM_SIZE: usize = 1024 * 1024; // 1MB

const HEAD_OFFSET: usize = 128;
const TAIL_OFFSET: usize = 132;
const DATA_OFFSET: usize = 192;
const DATA_SIZE: usize = SHM_SIZE - DATA_OFFSET;

const LogEvent = extern struct {
    timestamp: i64,
    severity: [24]u8,
    source_ip: [24]u8,
    facility: [24]u8,
    message: [24]u8,
};

// Stable POSIX C FFI
extern "c" fn open(path: [*:0]const u8, flags: c_int, mode: c_uint) c_int;
extern "c" fn mmap(addr: ?*anyopaque, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: i64) ?*anyopaque;
extern "c" fn munmap(addr: *anyopaque, len: usize) c_int;
extern "c" fn close(fd: c_int) c_int;
extern "c" fn usleep(useconds: c_uint) c_int;
extern "c" fn time(t: *i64) i64;

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

    const head_ptr: *align(4) std.atomic.Value(u32) = @ptrCast(@alignCast(mmap_slice + HEAD_OFFSET));
    const tail_ptr: *align(4) std.atomic.Value(u32) = @ptrCast(@alignCast(mmap_slice + TAIL_OFFSET));

    while (true) {
        const head = head_ptr.load(.acquire);
        const tail = tail_ptr.load(.acquire);

        const event_size = @sizeOf(LogEvent);
        var write_pos = head;
        
        // Wrap logic
        if (write_pos + event_size > DATA_SIZE) {
            write_pos = 0;
        }

        // Simplistic: check if we would overwrite tail
        // In a real ring buffer, we'd need to be more careful
        if (write_pos + event_size <= (if (tail <= write_pos) DATA_SIZE else tail)) {
            // Construct mock event
            var t: i64 = 0;
            var event = std.mem.zeroInit(LogEvent, .{
                .timestamp = time(&t),
                .severity = std.mem.zeroes([24]u8),
                .source_ip = std.mem.zeroes([24]u8),
                .facility = std.mem.zeroes([24]u8),
                .message = std.mem.zeroes([24]u8),
            });
            
            std.mem.copyForwards(u8, &event.severity, "INFO");
            std.mem.copyForwards(u8, &event.source_ip, "127.0.0.1");
            std.mem.copyForwards(u8, &event.facility, "auth");
            std.mem.copyForwards(u8, &event.message, "test log");
            
            const event_bytes: [*]const u8 = @ptrCast(&event);
            @memcpy(data_buffer[write_pos .. write_pos + event_size], event_bytes[0..event_size]);
            
            head_ptr.store(write_pos + @as(u32, @intCast(event_size)), .release);
        } else {
            _ = usleep(100); 
        }
    }
}
