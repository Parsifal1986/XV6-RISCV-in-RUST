use core::arch::{asm, global_asm};

use crate::main;
use crate::param::NCPU;
use crate::riscv::*;

// size of the per-CPU boot stack.
pub const STACKSIZE: usize = 4096 * 4;

global_asm!(include_str!("entry.S"), STACKSIZE = const STACKSIZE);

#[repr(C, align(16))]
pub struct Stack0([u8; STACKSIZE * NCPU]);

// entry.S needs one stack per CPU.
#[no_mangle]
pub static mut STACK0: Stack0 = Stack0([0; STACKSIZE * NCPU]);

// entry.S jumps here in machine mode on STACK0.
#[no_mangle]
pub extern "C" fn start() -> ! {
  // set M Previous Privilege mode to Supervisor, for mret.
  let mut x: u64 = r_mstatus();
  x &= !MSTATUS_MPP_MASK;
  x |= MSTATUS_MPP_S;
  w_mstatus(x);

  // set M Exception Program Counter to main, for mret.
  w_mepc(main as *const () as u64);

  // disable paging for now.
  w_satp(0);

  // delegate all interrupts and exceptions to supervisor mode.
  w_medeleg(0xffff);
  w_mideleg(0xffff);
  w_sie(r_sie() | SIE_SEIE | SIE_STIE);

  // configure Physical Memory Protection to give supervisor mode
  // access to all of physical memory.
  w_pmpaddr0(0x3fffffffffffff_u64);
  w_pmpcfg0(0xf);

  // enable hardware updates of page table A and D bits
  w_menvcfg(r_menvcfg() | MENVCFG_ADUE);

  // ask for clock interrupts.
  timerinit();

  // keep each CPU's hartid in its tp register, for cpuid().
  let id: u64 = r_mhartid();
  w_tp(id);

  // switch to supervisor mode and jump to main().
  unsafe {
    asm!("mret", options(noreturn));
  }
}

// ask each hart to generate timer interrupts.
fn timerinit() {
  // enable supervisor-mode timer interrupts.
  w_mie(r_mie() | MIE_STIE);

  // enable the sstc extension (i.e. stimecmp).
  w_menvcfg(r_menvcfg() | MENVCFG_STCE);

  // allow supervisor to use stimecmp and time.
  w_mcounteren(r_mcounteren() | 2);

  // ask for the very first timer interrupt.
  w_stimecmp(r_time() + 1000000);
}
