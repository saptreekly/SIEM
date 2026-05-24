.section .rodata
.align 2
month_days:
    .short 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334

.text
.global parse_timestamp_asm
.global _parse_timestamp_asm

# Optimized scalar timestamp parser
parse_timestamp_asm:
_parse_timestamp_asm:
    push %rbx
    push %r12
    push %r13
    push %r14
    
    # 1. Extract Year
    # YYYY
    movzbq (%rdi), %r8
    movzbq 1(%rdi), %r9
    movzbq 2(%rdi), %r10
    movzbq 3(%rdi), %r11
    sub $48, %r8
    sub $48, %r9
    sub $48, %r10
    sub $48, %r11
    imul $10, %r8
    add %r9, %r8
    imul $10, %r8
    add %r10, %r8
    imul $10, %r8
    add %r11, %r8 # %r8 = Year

    # 2. Extract Month (5, 6)
    movzbq 5(%rdi), %rax
    sub $48, %rax
    imul $10, %rax
    movzbq 6(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %rax # %rax = Month

    # 3. Extract Day (8, 9)
    movzbq 8(%rdi), %r9
    sub $48, %r9
    imul $10, %r9
    movzbq 9(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %r9 # %r9 = Day

    # 4. Extract HH (11, 12)
    movzbq 11(%rdi), %r10
    sub $48, %r10
    imul $10, %r10
    movzbq 12(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %r10 # %r10 = HH

    # 5. Extract MI (14, 15)
    movzbq 14(%rdi), %r11
    sub $48, %r11
    imul $10, %r11
    movzbq 15(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %r11 # %r11 = MI

    # 6. Extract SS (17, 18)
    movzbq 17(%rdi), %r12
    sub $48, %r12
    imul $10, %r12
    movzbq 18(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %r12 # %r12 = SS

    # 7. Math: Days = (Y-1970)*365 + (Y-1969)/4 + month_days[M-1] + D-1
    mov %r8, %rcx
    sub $1970, %rcx
    imul $365, %rcx, %rbx
    mov %r8, %rcx
    sub $1969, %rcx
    shr $2, %rcx
    add %rcx, %rbx
    
    # Month lookup
    lea month_days(%rip), %rcx
    movzwq -2(%rcx, %rax, 2), %rax
    add %rax, %rbx
    
    # Current leap year?
    cmp $2, %rax # Simplified check
    test $3, %r8
    jnz .no_leap
    add $1, %rbx
.no_leap:
    add %r9, %rbx
    dec %rbx

    # Total Epoch
    imul $86400, %rbx, %rax
    imul $3600, %r10, %rcx
    add %rcx, %rax
    imul $60, %r11, %rcx
    add %rcx, %rax
    add %r12, %rax
    
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    ret
