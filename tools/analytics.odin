package siem_analytics

import "core:fmt"
import "core:os"
import "core:mem"
import "core:time"
import "core:io"
import "core:encoding/endian"

SHM_SIZE :: 1024 * 1024; // 1MB
HEAD_OFFSET :: 0;
TAIL_OFFSET :: 4;
DATA_OFFSET :: 8;
DATA_SIZE :: SHM_SIZE - DATA_OFFSET;

// Needs to match Rust LogEvent structure
LogEvent :: struct {
    timestamp: i64,
    severity: [24]byte, // CompactString is complex, for prototype map to fixed size
    source_ip: [24]byte,
    facility: [24]byte,
    message: [24]byte,
}

main :: proc() {
    fd, err := os.open("/tmp/siem_shm.bin", os.O_RDWR)
    if err != 0 {
        fmt.eprintln("Error opening /tmp/siem_shm.bin:", os.error_string(err))
        return
    }
    defer os.close(fd)

    mmap, mmap_err := os.mmap_file(fd, 0, SHM_SIZE, {.Read, .Write})
    if mmap_err != 0 {
        fmt.eprintln("Error mmapping /tmp/siem_shm.bin:", os.error_string(mmap_err))
        return
    }
    defer os.munmap(mmap)

    data_buffer := mmap[DATA_OFFSET:]
    
    // Dynamic optimization window
    hot_window := make([dynamic]^LogEvent)
    defer delete(hot_window)

    fmt.println("Odin Analytics Engine: Started reading raw structs from /tmp/siem_shm.bin")
    
    tail := u32(0)
    for {
        head := endian.read_u32_le(mmap[HEAD_OFFSET:HEAD_OFFSET+4])
        tail = endian.read_u32_le(mmap[TAIL_OFFSET:TAIL_OFFSET+4])

        if (head == tail) {
            time.sleep(10 * time.Millisecond)
            continue
        }

        // Direct pointer cast from buffer
        event_ptr := (^LogEvent)(&data_buffer[tail])
        append(&hot_window, event_ptr)
        
        fmt.printf("Processed Event: TS=%d
", event_ptr.timestamp)
        
        // Advance tail
        tail = (tail + u32(mem.size_of(LogEvent))) % u32(DATA_SIZE)
        endian.write_u32_le(mmap[TAIL_OFFSET:TAIL_OFFSET+4], tail)
    }
}
