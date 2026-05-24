.section .rodata
.align 2
month_days:
    .short 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334

.text
.global parse_timestamp_asm
.global _parse_timestamp_asm

# i64 parse_timestamp_asm(const char *x0)
parse_timestamp_asm:
_parse_timestamp_asm:
    # x0 = input pointer
    # Extract Year (Indices 0-3)
    ldrb w2, [x0]
    sub w2, w2, #48
    mov w3, #1000
    mul w2, w2, w3
    mov w4, w2
    ldrb w2, [x0, #1]
    sub w2, w2, #48
    mov w3, #100
    mul w2, w2, w3
    add w4, w4, w2
    ldrb w2, [x0, #2]
    sub w2, w2, #48
    mov w3, #10
    mul w2, w2, w3
    add w4, w4, w2
    ldrb w2, [x0, #3]
    sub w2, w2, #48
    add w4, w4, w2
    # w4 = Year

    # Extract Month (5-6)
    ldrb w2, [x0, #5]
    sub w2, w2, #48
    mov w3, #10
    mul w2, w2, w3
    ldrb w5, [x0, #6]
    sub w5, w5, #48
    add w2, w2, w5
    # w2 = Month

    # Extract Day (8-9)
    ldrb w3, [x0, #8]
    sub w3, w3, #48
    mov w5, #10
    mul w3, w3, w5
    ldrb w5, [x0, #9]
    sub w5, w5, #48
    add w3, w3, w5
    # w3 = Day

    # Extract HH, MI, SS (11, 14, 17)
    ldrb w5, [x0, #11]
    sub w5, w5, #48
    mov w6, #10
    mul w5, w5, w6
    ldrb w6, [x0, #12]
    sub w6, w6, #48
    add w5, w5, w6
    mov w6, w5 # w6 = Hour

    ldrb w5, [x0, #14]
    sub w5, w5, #48
    mov w7, #10
    mul w5, w5, w7
    ldrb w7, [x0, #15]
    sub w7, w7, #48
    add w5, w5, w7
    mov w7, w5 # w7 = Min

    ldrb w5, [x0, #17]
    sub w5, w5, #48
    mov w8, #10
    mul w5, w5, w8
    ldrb w8, [x0, #18]
    sub w8, w8, #48
    add w5, w5, w8
    mov w8, w5 # w8 = Sec

    # Calc Days: (Year - 1970) * 365 + (Year - 1969) / 4
    sub w9, w4, #1970
    mov w10, #365
    mul w9, w9, w10
    sub w10, w4, #1969
    lsr w10, w10, #2
    add w9, w9, w10
    # w9 = Days base

    # Month offset
    adrp x10, month_days
    add x10, x10, :lo12:month_days
    sub w11, w2, #1
    add x10, x10, x11, lsl #1
    ldrh w11, [x10]
    add w9, w9, w11

    # Leap adjustment
    cmp w2, #2
    bls .no_leap
    tst w4, #3
    bne .no_leap
    add w9, w9, #1
.no_leap:
    add w9, w9, w3
    sub w9, w9, #1

    # Final: Days * 86400 + H*3600 + M*60 + S
    mov x10, #86400
    mul x10, x9, x10
    mov x11, #3600
    mul x11, x6, x11
    add x10, x10, x11
    mov x11, #60
    mul x11, x7, x11
    add x10, x10, x11
    add x10, x10, x8
    mov x0, x10
    ret
