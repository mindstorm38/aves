/* 
 * Linker Script for RISC-V QEMU "virt" machine. 
 * Théo Rozier - 2022
 */

/*
 * Architecture shown when using 'objdump -f'.
 */
OUTPUT_ARCH("riscv")

/*
 * The entry point of the kernel.
 */
ENTRY(_start)

/*
 * Define a (w) writeable, (x) executable, (a) allocatable memory
 * section. From the "virt" machine specification, the memory starts
 * at 
 */
MEMORY {
    ram (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

/*
 * Program Headers:
 * text - CPU instructions (executable sections)
 * data - Global, initialized variables
 * bss  - Global, uninitialized variables (all will be set to 0 at boot)
 */
PHDRS {
    text PT_LOAD;
    data PT_LOAD;
    bss PT_LOAD;
}

/*
 * We are now going to organize the memory based on which
 * section it is in. In assembly, we can change the section
 * with the ".section" directive. However, in C++ and Rust,
 * CPU instructions go into text, global constants go into
 * rodata, global initialized variables go into data, and
 * global uninitialized variables go into bss.
 */
SECTIONS {

    /*
     * The first part of our RAM layout will be the text section.
	 * Since our CPU instructions are here, and our memory starts at
	 * 0x8000_0000, we need our entry point to line up here.
     */
    
    .text : {

        /* 
         * PROVIDE allows me to access a symbol called _text_start so
         * I know where the text section starts in the operating system.
         * This should not move, but it is here for convenience.
         * The period '.' tells the linker to set _text_start to the
         * CURRENT location ('.' = current memory location). This current
         * memory location moves as we add things.
         */
        _text_start = .;
        *(.text.init) 
        *(.text .text.*)
        _text_end = .;

    } >ram AT>ram :text

    /*
     * The global pointer allows the linker to position global variables and constants into
     * independent positions relative to the gp (global pointer) register. The globals start
     * after the text sections and are only relevant to the rodata, data, and bss sections.
     */
    _global_pointer = .;

    .rodata : {
        _rodata_start = .;
        *(.rodata .rodata.*)
        _rodata_end = .;
    } >ram AT>ram :text

    .data : {

        /*
         *. = ALIGN(4096) tells the linker to align the current memory location (which is
         * 0x8000_0000 + text section + rodata section) to 4096 bytes. This is because our paging
         * system's resolution is 4,096 bytes or 4 KiB.
         */
        . = ALIGN(4096);

        _data_start = .;
        *(.sdata .sdata.*) 
        *(.data .data.*)
        _data_end = .;

    } >ram AT>ram :data

    .bss : {
        _bss_start = .;
        *(.sbss .sbss.*) 
        *(.bss .bss.*)
        _bss_end = .;
    } >ram AT>ram :bss

    _memory_start = ORIGIN(ram);
    _memory_end = ORIGIN(ram) + LENGTH(ram);
    _memory_size = LENGTH(ram);

    _kstack_start = _bss_end;
    _kstack_end = _kstack_start + 0x80000;

    _heap_start = _kstack_end;
    _heap_size = _memory_end - _heap_start;

}
