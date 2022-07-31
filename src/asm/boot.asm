# Definition of entry point.

.option norvc
.section .text.init

.global _start
_start:

    # Disable linker instruction relaxation for the `la` instruction below.
    # This disallows the assembler from assuming that `gp` is already initialized.
    # This causes the value stored in `gp` to be calculated from `pc`.
    # The job of the global pointer is to give the linker the ability to address
    # memory relative to GP instead of as an absolute address.
.option push
.option norelax
    la gp, _global_pointer
.option pop

    # Should be zero.
    csrw satp, zero

    # Load the hard id to t0.
    csrr t0, mhartid
    # If the hard id is not 0 (our bootstrapping hart), wait for interrupt.
    bnez t0, asm_abort

    # Here we want to init all the bss section to zero.
    la a0, _bss_start
    la a1, _bss_end
    bgeu a0, a1, clear_bss_end
clear_bss_loop:
    sd zero, (a0)
    addi a0, a0, 8
    bltu a0, a1, clear_bss_loop
clear_bss_end:

    # Setup stack, the stack grows from bottom to top.
    la sp, _kstack_end

    li t0, (3 << 11) | (1 << 7) | (1 << 3)
    csrw mstatus, t0

    # Load address of kmain entry point.
    la t0, kmain
    csrw mepc, t0

    # Set the trap vector address.
    la t0, asm_trap_vector
    csrw mtvec, t0

    # Enable interrupts.
    li t0, (1 << 3) | (1 << 7) | (1 << 11)
    csrw mie, t0
    
    # Abort after kmain returned.
    la ra, asm_abort

    # Actually go to kmain (jump to address into mepc).
    mret

# Define a global abort function, also used to park unused harts and for panics.
.global asm_abort
asm_abort:
    wfi
    j asm_abort
