const std = @import("std");

pub fn main() !void {
    const args = [_][]const u8{
        "/usr/bin/security",
        "find-generic-password",
        "-a", "siem_user",
        "-s", "siem_db_key",
        "-w"
    };

    var child = std.process.Child.init(&args, std.heap.page_allocator) catch |err| {
        std.debug.print("Failed to init child: {}
", .{err});
        std.process.exit(1);
    };
    child.stdout_behavior = .Pipe;
    
    try child.spawn();
    
    var buf: [1024]u8 = undefined;
    const n = try child.stdout.?.read(&buf);
    
    // Print just the key (trimming the trailing newline)
    const key = std.mem.trimRight(u8, buf[0..n], "
");
    std.io.getStdOut().writer().writeAll(key) catch {};
    
    _ = try child.wait();
}
