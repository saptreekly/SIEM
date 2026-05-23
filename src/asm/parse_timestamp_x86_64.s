.global _parse_timestamp_asm
_parse_timestamp_asm:
    # rdi = pointer to "YYYY-MM-DDTHH:MM:SSZ"
    # Return rax = epoch
    push %rbx
    push %r12
    
    # Extract YYYY (2026)
    movzbq (%rdi), %rax
    sub $48, %rax
    imul $1000, %rax
    movzbq 1(%rdi), %rbx
    sub $48, %rbx
    imul $100, %rbx
    add %rbx, %rax
    movzbq 2(%rdi), %rbx
    sub $48, %rbx
    imul $10, %rbx
    add %rbx, %rax
    movzbq 3(%rdi), %rbx
    sub $48, %rbx
    add %rbx, %rax
    # rax = Year
    
    # Simplified Epoch Calc: (Year - 1970) * 31536000
    sub $1970, %rax
    imul $31536000, %rax
    
    pop %r12
    pop %rbx
    ret
