.global fnv1a_hash_asm
.global _fnv1a_hash_asm
fnv1a_hash_asm:
_fnv1a_hash_asm:
    # rdi = pointer to data
    # rsi = length
    movl $0x811c9dc5, %eax
    movl $0x01000193, %edx
    testq %rsi, %rsi
    jz 2f
1:
    movzbq (%rdi), %rcx
    xorl %ecx, %eax
    imull %edx, %eax
    incq %rdi
    decq %rsi
    jnz 1b
2:
    ret
