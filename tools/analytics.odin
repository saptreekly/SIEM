package siem_analytics

import "core:fmt"
import "core:os"
import "core:mem"
import "core:time"
import "core:io"
import "core:encoding/endian"

SHM_SIZE :: 1024 * 1024; // 1MB, must match Zig
HEAD_OFFSET :: 0;
TAIL_OFFSET :: 4;
DATA_OFFSET :: 8;
DATA_SIZE :: SHM_SIZE - DATA_OFFSET;

main :: proc() {
    fd, err := os.open("/tmp/siem_shm.bin", os.O_RDWR) // Open for read/write
    if err != 0 {
        fmt.eprintln("Error opening /tmp/siem_shm.bin:", os.error_string(err))
        return
    }
    defer os.close(fd)

    mmap, mmap_err := os.mmap_file(fd, 0, SHM_SIZE, {.Read, .Write}) // Map as read/write
    if mmap_err != 0 {
        fmt.eprintln("Error mmapping /tmp/siem_shm.bin:", os.error_string(mmap_err))
        return
    }
    defer os.munmap(mmap)

    data_buffer := mmap[DATA_OFFSET:] // Slice from DATA_OFFSET to end

    head := u32(0);
    tail := u32(0);

    fmt.println("Odin Analytics Engine: Started reading from /tmp/siem_shm.bin")
    for {
        // Read head and tail from shared memory
        head = endian.read_u32_le(mmap[HEAD_OFFSET:HEAD_OFFSET+4])
        tail = endian.read_u32_le(mmap[TAIL_OFFSET:TAIL_OFFSET+4])

        if (head == tail) {
            // Buffer is empty, wait and retry
            time.sleep(10 * time.Millisecond)
            continue
        }

        if (head > tail) {
            // Read in one go
            fmt.print(string(data_buffer[tail:head]))
            tail = head;
        } else {
            // Read to end, then from beginning
            fmt.print(string(data_buffer[tail:DATA_SIZE]))
            fmt.print(string(data_buffer[0:head]))
            tail = head;
        }

        // Write new tail to shared memory
        endian.write_u32_le(mmap[TAIL_OFFSET:TAIL_OFFSET+4], tail)

        // No sleep here, read as fast as possible
    }
}
