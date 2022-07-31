# Aves

Based on tutorial at https://github.com/sgmarz/osblog

## Based on
- [Primary tutorial](https://github.com/sgmarz/osblog)
- [Linker script manual](https://users.informatik.haw-hamburg.de/~krabat/FH-Labor/gnupro/5_GNUPro_Utilities/c_Using_LD/ldLinker_scripts.html)
- [RISC-V standard specification](https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMAFDQC/riscv-spec-20191213.pdf)
- [RISC-V privileged specification](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)
- [RISC-V programmer's manual](https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md)
- [RISC-V calling convention](https://riscv.org/wp-content/uploads/2015/01/riscv-calling.pdf)

## Notes
All harts (cores) in the processor start at the entry point at the same time.
For now, we only start the hart #0 and park others.
