use crate::riscv::{PGSIZE, MAXVA};

pub const UART0: u64 = 0x1000_0000;
pub const UART0_IRQ: u32 = 10;

pub const VIRTIO0: u64 = 0x1000_1000;
pub const VIRTIO0_IRQ: u32 = 1;

pub const PLIC: u64 = 0x0c00_0000;
pub const PLIC_PRIORITY: u64 = PLIC + 0x0;
pub const PLIC_PENDING: u64 = PLIC + 0x1000;

#[inline(always)]
pub const fn plic_senable(hart: u64) -> u64 {
    PLIC + 0x2080 + hart * 0x100
}

#[inline(always)]
pub const fn plic_spriority(hart: u64) -> u64 {
    PLIC + 0x201_000 + hart * 0x2000
}

#[inline(always)]
pub const fn plic_sclaim(hart: u64) -> u64 {
    PLIC + 0x201_004 + hart * 0x2000
}

pub const KERNBASE: u64 = 0x8000_0000;

pub const PHYSTOP: u64 = KERNBASE + 128 * 1024 * 1024;

pub const TRAMPOLINE: u64 = MAXVA - PGSIZE;

#[inline(always)]
pub const fn kstack(p: u64) -> u64 {
    TRAMPOLINE - (p + 1) * 2 * PGSIZE
}

pub const TRAPFRAME: u64 = TRAMPOLINE - PGSIZE;
