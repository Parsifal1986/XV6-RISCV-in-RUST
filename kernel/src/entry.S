        # qemu -kernel loads the kernel at 0x80000000
        # and causes each hart (i.e. CPU) to jump there.
        # kernel.ld causes the following code to
        # be placed at 0x80000000.
.section .text.entry
.global _entry
_entry:
        .option arch, +m
        # set up a stack for Rust.
        # STACK0 is declared in start.rs,
        # with a STACKSIZE-byte stack per CPU.
        # sp = STACK0 + ((hartid + 1) * STACKSIZE)
        la sp, STACK0
        li a0, {STACKSIZE}
        csrr a1, mhartid
        addi a1, a1, 1
        mul a0, a0, a1
        add sp, sp, a0
        # jump to start() in start.rs
        call start
spin:
        j spin
