// Re-exportation of linker symbols

.section .rodata

.global LD_MEMORY_START
.global LD_MEMORY_END
.global LD_MEMORY_SIZE
LD_MEMORY_START: .dword _ld_memory_start
LD_MEMORY_END: .dword _ld_memory_end
LD_MEMORY_SIZE: .dword _ld_memory_size

.global LD_KSTACK_START
.global LD_KSTACK_END
LD_KSTACK_START: .dword _ld_kstack_start
LD_KSTACK_END: .dword _ld_kstack_end

.global LD_HEAP_START
.global LD_HEAP_SIZE
LD_HEAP_START: .dword _ld_heap_start
LD_HEAP_SIZE: .dword _ld_heap_size
