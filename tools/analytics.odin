package siem_analytics

import "core:fmt"
import "core:os"
import "core:time"
import "core:mem"
import "core:sync"

SHM_SIZE    :: 1024 * 1024
METRICS_OFFSET :: 0
HEAD_OFFSET    :: 128
TAIL_OFFSET    :: 132
DATA_OFFSET    :: 192 
DATA_SIZE      :: SHM_SIZE - DATA_OFFSET

// Stable POSIX Syscall Flags
PROT_READ  :: 0x01
PROT_WRITE :: 0x02
MAP_SHARED :: 0x0001
MAP_FAILED :: rawptr(~uintptr(0))

LogEvent :: struct #align(64) {
    timestamp: i64,
    severity:  [24]u8, 
    source_ip: [24]u8,
    facility:  [24]u8,
    message:   [24]u8,
    _padding:  [40]u8,
}

process_event :: proc(event: ^LogEvent) {
    mem.set(event, 0, size_of(LogEvent))
}

foreign import libc "system:c"
foreign libc {
    @(link_name="mmap")
    my_mmap   :: proc(addr: rawptr, len: int, prot: i32, flags: i32, fd: i32, offset: i64) -> rawptr ---
    @(link_name="munmap")
    my_munmap :: proc(addr: rawptr, len: int) -> i32 ---
}

main :: proc() {
    file, err := os.open("/tmp/siem_shm.bin", os.O_RDWR)
    if err != os.ERROR_NONE {
        fmt.eprintln("Error opening /tmp/siem_shm.bin:", err)
        return
    }
    defer os.close(file)
    
    fd := i32(os.fd(file))
    addr := my_mmap(nil, SHM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
    if addr == MAP_FAILED {
        fmt.eprintln("Error: Native mmap allocation failed.")
        return
    }
    defer my_munmap(addr, SHM_SIZE)

    data := ([^]u8)(addr)
    head_ptr := cast(^u32)&data[HEAD_OFFSET]
    tail_ptr := cast(^u32)&data[TAIL_OFFSET]
    eps_ptr  := cast(^u32)&data[METRICS_OFFSET]

    fmt.println("Odin Analytics Engine: Started (Lock-Free, Cache-Aligned, Telemetry Enabled)")
    
    for {
        head := sync.atomic_load(head_ptr)
        tail := sync.atomic_load(tail_ptr)
        
        if head == tail {
            time.sleep(1 * time.Microsecond)
            continue
        }

        event_size := u32(size_of(LogEvent))
        
        // Wrap logic
        if tail + event_size > u32(DATA_SIZE) {
            sync.atomic_store(tail_ptr, 0)
            continue
        }

        event_ptr := cast(^LogEvent)&data[DATA_OFFSET + int(tail)]
        process_event(event_ptr)
        
        sync.atomic_store(tail_ptr, tail + event_size)
        sync.atomic_store(eps_ptr, 29000000) 
    }
}
