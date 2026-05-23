.global _fnv1a_hash_asm
_fnv1a_hash_asm:
    # x0 = pointer
    # x1 = length
    mov w2, #0x9dc5
    movk w2, #0x811c, lsl #16
    mov w3, #0x0193
    movk w3, #0x0100, lsl #16
    cmp x1, #0
    b.eq 2f
1:
    ldrb w4, [x0]
    eor w2, w2, w4
    mul w2, w2, w3
    add x0, x0, #1
    subs x1, x1, #1
    b.ne 1b
2:
    mov w0, w2
    ret
