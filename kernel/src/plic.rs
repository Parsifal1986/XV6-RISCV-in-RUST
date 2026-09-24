//
// the riscv Platform Level Interrupt Controller (PLIC).
//

use core::ptr::{read_volatile, write_volatile};

use crate::memlayout::{plic_sclaim, plic_senable, plic_spriority, PLIC, UART0_IRQ, VIRTIO0_IRQ};
use crate::proc::cpuid;

pub fn plicinit() {
  unsafe {
    // set desired IRQ priorities non-zero (otherwise disabled).
    write_volatile((PLIC + UART0_IRQ as u64 * 4) as *mut u32, 1);
    write_volatile((PLIC + VIRTIO0_IRQ as u64 * 4) as *mut u32, 1);
  }
}

pub fn plicinithart() {
  let hart = cpuid() as u64;

  unsafe {
    // set enable bits for this hart's S-mode
    // for the uart and virtio disk.
    write_volatile(plic_senable(hart) as *mut u32, (1 << UART0_IRQ) | (1 << VIRTIO0_IRQ));

    // set this hart's S-mode priority threshold to 0.
    write_volatile(plic_spriority(hart) as *mut u32, 0);
  }
}

// ask the PLIC what interrupt we should serve.
pub fn plic_claim() -> u32 {
  let hart = cpuid() as u64;
  unsafe { read_volatile(plic_sclaim(hart) as *const u32) }
}

// tell the PLIC we've served this IRQ.
pub fn plic_complete(irq: u32) {
  let hart = cpuid() as u64;
  unsafe { write_volatile(plic_sclaim(hart) as *mut u32, irq) };
}
