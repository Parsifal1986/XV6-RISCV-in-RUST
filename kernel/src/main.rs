#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(static_mut_refs)]
#![allow(clippy::missing_safety_doc)]

use core::sync::atomic::{AtomicBool, Ordering};

use crate::bio::binit;
use crate::console::consoleinit;
use crate::file::fileinit;
use crate::fs::iinit;
use crate::kalloc::kinit;
use crate::plic::{plicinit, plicinithart};
use crate::printf::printfinit;
use crate::proc::{cpuid, procinit, scheduler, userinit};
use crate::trap::{trapinit, trapinithart};
use crate::virtio_disk::virtio_disk_init;
use crate::vm::{kvminit, kvminithart};

#[macro_use]
mod printf;
mod bio;
mod buf;
mod console;
mod elf;
mod exec;
mod fcntl;
mod file;
mod fs;
mod kalloc;
mod log;
mod memlayout;
mod param;
mod pipe;
mod plic;
mod proc;
mod riscv;
mod sleeplock;
mod spinlock;
mod start;
mod stat;
mod string;
mod syscall;
mod sysfile;
mod sysproc;
mod trap;
mod uart;
mod virtio;
mod virtio_disk;
mod vm;

static STARTED: AtomicBool = AtomicBool::new(false);

// start() jumps here in supervisor mode on all CPUs.
#[no_mangle]
pub extern "C" fn main() -> ! {
  if cpuid() == 0 {
    consoleinit();
    printfinit();
    printf!("\n");
    printf!("xv6 kernel is booting\n");
    printf!("\n");
    kinit();            // physical page allocator
    kvminit();          // create kernel page table
    kvminithart();      // turn on paging
    procinit();         // process table
    trapinit();         // trap vectors
    trapinithart();     // install kernel trap vector
    plicinit();         // set up interrupt controller
    plicinithart();     // ask PLIC for device interrupts
    binit();            // buffer cache
    iinit();            // inode table
    fileinit();         // file table
    virtio_disk_init(); // emulated hard disk
    userinit();         // first user process

    STARTED.store(true, Ordering::Release);
  } else {
    while !STARTED.load(Ordering::Acquire) {
      core::hint::spin_loop();
    }

    printf!("hart {} starting\n", cpuid());
    kvminithart();  // turn on paging
    trapinithart(); // install kernel trap vector
    plicinithart(); // ask PLIC for device interrupts
  }

  scheduler();
}
