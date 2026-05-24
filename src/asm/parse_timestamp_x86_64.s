.section .rodata
.align 2
month_days:
    .short 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334

.text
.global parse_timestamp_asm
.global _parse_timestamp_asm

# i64 parse_timestamp_asm(const char *rdi)
parse_timestamp_asm:
_parse_timestamp_asm:
    # Save callee-saved registers
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15
    
    # Extract Year (Indices 0-3)
    movzbq 0(%rdi), %r8
    sub $48, %r8
    imul $1000, %r8
    movzbq 1(%rdi), %rax
    sub $48, %rax
    imul $100, %rax
    add %rax, %r8
    movzbq 2(%rdi), %rax
    sub $48, %rax
    imul $10, %rax
    add %rax, %r8
    movzbq 3(%rdi), %rax
    sub $48, %rax
    add %rax, %r8
    
    # Extract Month (Indices 5-6)
    movzbq 5(%rdi), %r9
    sub $48, %r9
    imul $10, %r9
    movzbq 6(%rdi), %rax
    sub $48, %rax
    add %rax, %r9
    
    # Extract Day (Indices 8-9)
    movzbq 8(%rdi), %r10
    sub $48, %r10
    imul $10, %r10
    movzbq 9(%rdi), %rax
    sub $48, %rax
    add %rax, %r10
    
    # Extract Hour (Indices 11-12)
    movzbq 11(%rdi), %r11
    sub $48, %r11
    imul $10, %r11
    movzbq 12(%rdi), %rax
    sub $48, %rax
    add %rax, %r11
    
    # Extract Minute (Indices 14-15)
    movzbq 14(%rdi), %r12
    sub $48, %r12
    imul $10, %r12
    movzbq 15(%rdi), %rax
    sub $48, %rax
    add %rax, %r12
    
    # Extract Second (Indices 17-18)
    movzbq 17(%rdi), %r13
    sub $48, %r13
    imul $10, %r13
    movzbq 18(%rdi), %rax
    sub $48, %rax
    add %rax, %r13
    
    # Days since 1970-01-01
    # r14 = (Year - 1970) * 365 + (Year - 1969) / 4
    # (Year - 1969) / 4 counts leap years between 1970 and Year-1
    mov %r8, %rax
    sub $1970, %rax
    imul $365, %rax, %r14
    mov %r8, %rax
    sub $1969, %rax
    shr $2, %rax
    add %rax, %r14
    
    # Add days before current month using lookup table
    lea month_days(%rip), %rcx
    movzwq -2(%rcx, %r9, 2), %rax # r9 is 1-indexed (1-12)
    add %rax, %r14
    
    # Leap adjustment for current year: if Month > 2 and Year % 4 == 0
    cmp $2, %r9
    jbe .no_leap_adj
    test $3, %r8
    jnz .no_leap_adj
    inc %r14
.no_leap_adj:
    
    # Add day of month - 1 (for 0-indexed day offset)
    add %r10, %r14
    dec %r14
    
    # Total Epoch Seconds = Days * 86400 + Hour * 3600 + Min * 60 + Sec
    imul $86400, %r14, %rax
    imul $3600, %r11, %rbx
    add %rbx, %rax
    imul $60, %r12, %rbx
    add %rbx, %rax
    add %r13, %rax
    
    # Restore callee-saved registers
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    ret
