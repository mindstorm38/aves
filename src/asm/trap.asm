# Definition of assembly trap vector.

.option norvc
.section .text

.global asm_trap_vector
.align 4
asm_trap_vector:

#     # Atomically swap values between t0 and mscratch.
#     # The scratch CSR must contain a pointer to a 'TrapFrame'
#     # structure.
#     csrrw t0, mscratch, t0

#     # Here, t0 is a pointer to a 'TrapFrame', so lets save
#     # t1 and t2 in the frame.
#     sd 8(t0), t1
#     sd 16(t0), t2

#     # Copy t0 (pointer to 'TrapFrame') to t2.
#     mv t2, t0
#     # Restore old t0 from mscratch, and set mscratch to t2 
#     # (pointer to 'TrapFrame').
#     csrrw t0, mscratch, t2
#     sd 0(t2), t0

#     # From here, we are free to use t0 and t2 as we want.
#     # t2 is used for the 'TrapFrame'.

#     # Load 'TrapFrame.traps_count' to t0.
#     ld t0, 24(t2)

#     # Saves 'TrapFrame.traps_count += 1'.
#     addi t0, t0, 1
#     sd 24(t2), t0
    
#     # Shift left to multiply the trap count by 24 (16 + 8, sizeof 'Trap').
#     slli t0, t0, 4
#     slli t1, t0, 3
#     add t0, t0, t1

#     # Now pointing t1 to 'TrapFrame.traps[t0]' - 32 (offset of the array).
#     addi t1, t2, t0

#     # Store values in the 'Trap' object.
#     csrr t0, mcause
#     sd 32(t1), t0
#     csrr t0, mtval
#     sd 40(t1), t0
#     csrr t0, mepc
#     sd 48(t1), t0

#     andi t1, t2, 1 << 63
#     bnez t1, is_interrupt
    
# is_exception:

#     # We increment mepc by 4 in order to skip the exception instruction.
#     addi t0, t0, 4
#     csrw mepc, t0

# is_interrupt:

#     # Restore registers.
#     ld t0, 0(t2)
#     ld t1, 8(t2)
#     ld t2, 16(t2)

    mret
