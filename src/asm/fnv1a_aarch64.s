.global fnv1a_hash_asm
fnv1a_hash_asm:
    # x0 = pointer
    # x1 = length
    ldr w2, =0x811c9dc5
    ldr w3, =0x01000193
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
