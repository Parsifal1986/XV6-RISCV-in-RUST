#![no_std]
#![no_main]

use crate::{console::consoleinit, kalloc::kinit, printf::printfinit, proc::cpuid, vm::{kvminit, kvminithart}};

mod vm;
mod riscv;
mod start;
mod kalloc;
mod spinlock;
mod proc;
mod param;
mod memlayout;
mod defs;
mod types;
mod console;
mod uart;
mod printf;
mod exec;
mod elf;
mod file;
mod fs;
mod sleeplock;
mod pipe;
mod log;
mod buf;
mod stat;
mod syscall;
mod trap;
mod plic;
mod virtio_disk;
mod sysfile;
mod sysproc;

fn main() {
  if cpuid() == 0 {
    consoleinit();
    printfinit();
    kinit();
    kvminit();
    kvminithart();
  }
}
