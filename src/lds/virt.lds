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
 * Program Headers,
 */
PHDRS {

}