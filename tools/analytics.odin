package siem_analytics

import "core:fmt"
import "core:os"
import "core:time"
import "core:mem"

SHM_SIZE    :: 1024 * 1024
HEAD_OFFSET :: 0
TAIL_OFFSET :: 4
DATA_OFFSET :: 8
DATA_SIZE   :: SHM_SIZE - DATA_OFFSET

// Stable POSIX Syscall Flags
PROT_READ  :: 0x01
PROT_WRITE :: 0x02
MAP_SHARED :: 0x0001
MAP_FAILED :: rawptr(~uintptr(0))

@(builtin)
LogEvent :: struct #align(4) {
    timestamp: i64,
    severity:  [24]u8, 
    source_ip: [24]u8,
    facility:  [24]u8,
    message:   [24]u8,
}

// Zero-copy event processing
process_event :: proc(event: ^LogEvent) {
    // Process data in-place
    fmt.printf("Processed Event: TS=%d, Msg=%s\n", event.timestamp, string(event.message[:]))
    
    // Clear slot for re-use
    mem.set(event, 0, size_of(LogEvent))
}

// Bind directly to macOS system libc to bypass Odin standard library discrepancies
// Using unique names to avoid collisions with core:sys/posix
foreign import libc "system:c"

foreign libc {
    @(link_name="mmap")
    my_mmap   :: proc(addr: rawptr, len: int, prot: i32, flags: i32, fd: i32, offset: i64) -> rawptr ---
    @(link_name="munmap")
    my_munmap :: proc(addr: rawptr, len: int) -> i32 ---
    @(link_name="sem_open")
    sem_open  :: proc(name: cstring, oflag: i32, mode: i32, value: i32) -> rawptr ---
    @(link_name="sem_wait")
    sem_wait  :: proc(sem: rawptr) -> i32 ---
    @(link_name="sem_post")
    sem_post  :: proc(sem: rawptr) -> i32 ---
    @(link_name="sem_close")
    sem_close :: proc(sem: rawptr) -> i32 ---
}

main :: proc() {
    // 1. Open file using Odin's wrapper to get a handle
    file, err := os.open("/tmp/siem_shm.bin", os.O_RDWR)
    if err != os.ERROR_NONE {
        fmt.eprintln("Error opening /tmp/siem_shm.bin:", err)
        return
    }
    defer os.close(file)
    
    // Get raw fd from the handle
    fd := i32(os.fd(file))

    // 2. Map file into process address space
    addr := my_mmap(nil, SHM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
    if addr == MAP_FAILED {
        fmt.eprintln("Error: Native macOS mmap allocation failed.")
        return
    }
    defer my_munmap(addr, SHM_SIZE)

    // Open semaphore
    // O_CREAT = 0x0200
    sem := sem_open("/siem_shm_sem", 0x0200, 0o666, 1)
    if sem == MAP_FAILED {
        fmt.eprintln("Error: Failed to open semaphore.")
        return
    }
    defer sem_close(sem)

    // 3. Cast raw pointer allocation to a safe byte index array slice
    data := ([^]u8)(addr)[:SHM_SIZE]

    fmt.println("Odin Analytics Engine: Started reading raw structs from /tmp/siem_shm.bin (Zero-Copy)")
    
    event_count := 0
    for event_count < 500 {
        sem_wait(sem)
        head_ptr := cast(^u32)&data[HEAD_OFFSET]
        tail_ptr := cast(^u32)&data[TAIL_OFFSET]
        
        head := head_ptr^
        tail := tail_ptr^
        
        if head == tail {
            sem_post(sem)
            time.sleep(10 * time.Millisecond)
            continue
        }

        // Handle wrap-around
        if int(tail) + size_of(LogEvent) > DATA_SIZE {
            tail = 0;
        }

        // Overlay our LogEvent structure blueprint exactly where the tail offset indicates
        event_ptr := cast(^LogEvent)&data[DATA_OFFSET + int(tail)]
        
        // Zero-copy processing
        process_event(event_ptr)
        
        // Step forward in memory cleanly by the uniform size of our data struct
        tail = (tail + u32(size_of(LogEvent))) % u32(DATA_SIZE)
        tail_ptr^ = tail
        sem_post(sem)

        event_count += 1
    }
}
