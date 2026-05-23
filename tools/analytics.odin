package siem_analytics

import "core:fmt"
import "core:os"
import "core:time"
import "core:sys/posix"

SHM_SIZE    :: 1024 * 1024
HEAD_OFFSET :: 0
TAIL_OFFSET :: 4
DATA_OFFSET :: 8
DATA_SIZE   :: SHM_SIZE - DATA_OFFSET

LogEvent :: struct {
    timestamp: i64,
    severity:  [24]u8, 
    source_ip: [24]u8,
    facility:  [24]u8,
    message:   [24]u8,
}

main :: proc() {
    fd, err := os.open("/tmp/siem_shm.bin", os.O_RDWR)
    if err != os.ERROR_NONE {
        fmt.eprintln("Error opening /tmp/siem_shm.bin:", err)
        return
    }
    defer os.close(fd)

    // Using POSIX constants from core:sys/posix
    addr, mmap_err := posix.mmap(
        nil, 
        SHM_SIZE, 
        posix.PROT_READ | posix.PROT_WRITE, 
        posix.MAP_SHARED, 
        i32(os.get_fd(fd)), 
        0,
    )
    if mmap_err != .NONE {
        fmt.eprintln("Error mmapping /tmp/siem_shm.bin:", mmap_err)
        return
    }
    defer posix.munmap(addr, SHM_SIZE)

    data := ([^]u8)(addr)[:SHM_SIZE]

    hot_window := make([dynamic]LogEvent)
    defer delete(hot_window)

    fmt.println("Odin Analytics Engine: Started reading raw structs from /tmp/siem_shm.bin")
    
    for {
        head_ptr := cast(^u32)&data[HEAD_OFFSET]
        tail_ptr := cast(^u32)&data[TAIL_OFFSET]
        
        head := head_ptr^
        tail := tail_ptr^

        if head == tail {
            time.sleep(10 * time.Millisecond)
            continue
        }

        event_ptr := cast(^LogEvent)&data[DATA_OFFSET + int(tail)]
        append(&hot_window, event_ptr^)
        
        fmt.printf("Processed Event: TS=%d\n", event_ptr.timestamp)
        
        tail = (tail + u32(size_of(LogEvent))) % u32(DATA_SIZE)
        tail_ptr^ = tail
    }
}
