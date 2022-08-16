# Aves

Based on tutorial at https://github.com/sgmarz/osblog

## Based on
- [Tutorial](https://osblog.stephenmarz.com/index.html)
- [Tutorial (Github)](https://github.com/sgmarz/osblog)
- [Linker script manual](https://users.informatik.haw-hamburg.de/~krabat/FH-Labor/gnupro/5_GNUPro_Utilities/c_Using_LD/ldLinker_scripts.html)
- [RISC-V standard specification](https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMAFDQC/riscv-spec-20191213.pdf)
- [RISC-V privileged specification](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)
- [RISC-V programmer's manual](https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md)
- [RISC-V calling convention](https://riscv.org/wp-content/uploads/2015/01/riscv-calling.pdf)
- [SiFive's Freedom Unleashed manual](https://sifive.cdn.prismic.io/sifive%2F834354f0-08e6-423c-bf1f-0cb58ef14061_fu540-c000-v1.0.pdf)

## Notes
All harts (cores) in the processor start at the entry point at the same time.
For now, we only start the hart #0 and park others.

In this OS, all processes will start on hart #0 in machine mode. 
Supervisor and user mode are not used, as well as the memory translation.

Before running the kernel, you will need to create a virtual HDD disk, without it qemu wouldn't launch: `dd if=/dev/zero of=hdd.dsk bs=32M count=1` in the project's directory.