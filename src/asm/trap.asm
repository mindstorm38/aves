# Definition of assembly trap vector.

.option norvc
.section .text

.global asm_trap_vector
.align 4
asm_trap_vector:

    # csrr a0, mcause
    # csrr a1, mtval

    # Save T0, our only register used in the following handler.
    csrw mscratch, t0

    # Compare mcause with 7 (Environment call from M-mode).
    addi t0, x0, 7
    beq mcause, t0, trap_machine_ecall

trap_ret:
    csrr t0, mscratch
    mret

trap_machine_ecall:
    j trap_ret
