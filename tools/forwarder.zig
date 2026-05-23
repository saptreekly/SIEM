const std = @import("std");

pub fn main() !void {
    var buffer: [65536]u8 = undefined;
    var fba = std.heap.FixedBufferAllocator.init(&buffer);
    const allocator = fba.allocator();

    const udp_address = try std.net.Address.parseIp("0.0.0.0", 514);
    var udp_socket = try std.net.udpServer(udp_address);
    defer udp_socket.close();

    const tcp_address = try std.net.Address.parseIp("127.0.0.1", 8080);

    var udp_buf: [65536]u8 = undefined;
    
    std.debug.print("Forwarder listening on UDP 514, forwarding to TCP 8080\n", .{});

    while (true) {
        var stream = std.net.tcpConnectToAddress(tcp_address) catch |err| {
            std.debug.print("Failed to connect to SIEM ({}). Retrying in 2 seconds...\n", .{err});
            std.time.sleep(2 * std.time.ns_per_s);
            continue;
        };
        defer stream.close();
        std.debug.print("Connected to SIEM\n", .{});

        while (true) {
            // Reset allocator for each packet to keep it zero-allocation on heap
            fba.reset();

            const result = udp_socket.receiveFrom(&udp_buf) catch |err| {
                std.debug.print("UDP receive error: {}\n", .{err});
                break;
            };

            const packet = udp_buf[0..result.numberOfBytes];
            
            // Format with newline using fixed allocator
            const framed = std.fmt.allocPrint(allocator, "{s}\n", .{packet}) catch |err| {
                std.debug.print("Buffer error: {}\n", .{err});
                continue;
            };

            stream.writer().writeAll(framed) catch |err| {
                std.debug.print("TCP write error: {}\n", .{err});
                break;
            };
        }
        std.debug.print("Connection lost. Reconnecting...\n", .{});
    }
}
