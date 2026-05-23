.global _parse_timestamp_asm
_parse_timestamp_asm:
    # x0 = pointer
    # Output: x0 = timestamp
    
    # Extract YYYY
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
    
    # Calc: (Year - 1970) * 31536000
    sub w4, w4, #1970
    ldr w2, =31536000
    mul w0, w4, w2
    
    ret
