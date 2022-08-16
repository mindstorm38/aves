# Definition of low-level function for context 
# switching between processes.

.option norvc
.section .text

.global asm_process_switch           # (to: *mut Process, exit_fn: extern "C" fn() -> !, from: *mut Process)
.global asm_process_switch_noreturn  # (to: *mut Process, exit_fn: extern "C" fn() -> !)

asm_process_switch:
save_from__:
    # We put 'ra' (return address) of the current
    # function as the program counter to restore.
    sd ra, 32(a2)
    sd sp, 40(a2)
    # unused: 48(a2)
    sd s0, 56(a2)
    sd s1, 64(a2)
    sd s2, 72(a2)
    sd s3, 80(a2)
    sd s4, 88(a2)
    sd s5, 96(a2)
    sd s6, 104(a2)
    sd s7, 112(a2)
    sd s8, 120(a2)
    sd s9, 128(a2)
    sd s10, 136(a2)
    sd s11, 144(a2)

asm_process_switch_noreturn:
restore_to__:
    # Here we need to restore the registers.
    ld sp, 40(a0)
    # unused: 48(a0)
    ld s0, 56(a0)
    ld s1, 64(a0)
    ld s2, 72(a0)
    ld s3, 80(a0)
    ld s4, 88(a0)
    ld s5, 96(a0)
    ld s6, 104(a0)
    ld s7, 112(a0)
    ld s8, 120(a0)
    ld s9, 128(a0)
    ld s10, 136(a0)
    ld s11, 144(a0)

test_spawned__:
    # If we switch to a newly spawned process,
    # we now that we will jump to the start of
    # a 'extern "C" fn()', so we set the 'ra'
    # (return address) of the function to the
    # 'exit' function given in parameter.
    li t0, 0x1      # t0 <- ProcessState::Spawned
    ld t1, 288(a0)  # t1 <- to.state
    bne t0, t1, state

spawned__:
    # Set return address to a1, which is 'exit_fn'.
    mv ra, a1

state:
    li t0, 0x3      # t0 <- ProcessState::Running
    sd t0, 288(a0)

switch__:
    # Jump to 'to.context.pc'.
    ld t0, 32(a0)
    jalr x0, t0, 0

    # This function intentionnaly has no return
    # instruction, because we will never get here
    # because we put the return address 'ra'
    # into the next PC to restore.
    # This function _will_ return, but from the 
    # 'jalr' instruction above, not a 'ret'.
