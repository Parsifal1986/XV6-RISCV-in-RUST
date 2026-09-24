use core::arch::global_asm;

use crate::memlayout::{KSTACKSIZE, TRAMPOLINE, TRAPFRAME, UART0_IRQ, VIRTIO0_IRQ};
use crate::plic::{plic_claim, plic_complete};
use crate::printf::panic;
use crate::proc::{cpuid, kexit, killed, myproc, setkilled, wakeup, yieldcpu};
use crate::riscv::{
  intr_get, intr_off, intr_on, r_satp, r_scause, r_sepc, r_sstatus, r_stval, r_time, r_tp, w_sepc, w_sstatus, w_stimecmp,
  w_stvec, MAKE_SATP, SSTATUS_SPIE, SSTATUS_SPP,
};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::syscall::syscall;
use crate::uart::uartintr;
use crate::virtio_disk::virtio_disk_intr;
use crate::vm::vmfault;

global_asm!(include_str!("kernelvec.S"));
global_asm!(include_str!("trampoline.S"), TRAPFRAME = const TRAPFRAME);

pub static mut TICKSLOCK: Spinlock = Spinlock::new();
pub static mut TICKS: u32 = 0;

extern "C" {
  // trampoline.S
  static trampoline: [u8; 0];
  static uservec: [u8; 0];

  // in kernelvec.S, calls kerneltrap().
  fn kernelvec();
}

pub fn trapinit() {
  initlock(&raw mut TICKSLOCK, "time");
}

// set up to take exceptions and traps while in the kernel.
pub fn trapinithart() {
  w_stvec(kernelvec as *const () as u64);
}

//
// handle an interrupt, exception, or system call from user space.
// called from, and returns to, trampoline.S
// return value is user satp for trampoline.S to switch to.
//
#[no_mangle]
pub extern "C" fn usertrap() -> u64 {
  let mut which_dev = 0;

  if r_sstatus() & SSTATUS_SPP != 0 {
    panic("usertrap: not from user mode");
  }

  // send interrupts and exceptions to kerneltrap(),
  // since we're now in the kernel.
  w_stvec(kernelvec as *const () as u64);

  let p = myproc();

  unsafe {
    // save user program counter.
    (*(*p).trapframe).epc = r_sepc();

    if r_scause() == 8 {
      // system call

      if killed(p) {
        kexit(-1);
      }

      // sepc points to the ecall instruction,
      // but we want to return to the next instruction.
      (*(*p).trapframe).epc += 4;

      // an interrupt will change sepc, scause, and sstatus,
      // so enable only now that we're done with those registers.
      intr_on();

      syscall();
    } else if {
      which_dev = devintr();
      which_dev
    } != 0
    {
      // ok
    } else if (r_scause() == 15 || r_scause() == 13) && vmfault((*p).pagetable, r_stval(), r_scause() == 13) != 0 {
      // page fault on lazily-allocated page
    } else {
      printf!("usertrap(): unexpected scause {:#x} pid={}\n", r_scause(), (*p).pid);
      printf!("            sepc={:#x} stval={:#x}\n", r_sepc(), r_stval());
      setkilled(p);
    }

    if killed(p) {
      kexit(-1);
    }

    // give up the CPU if this is a timer interrupt.
    if which_dev == 2 {
      yieldcpu();
    }

    prepare_return();

    // the user page table to switch to, for trampoline.S
    // return to trampoline.S; satp value in a0.
    MAKE_SATP((*p).pagetable as u64)
  }
}

//
// set up trapframe and control registers for a return to user space
//
pub fn prepare_return() {
  let p = myproc();

  // we're about to switch the destination of traps from
  // kerneltrap() to usertrap(). because a trap from kernel
  // code to usertrap would be a disaster, turn off interrupts.
  intr_off();

  unsafe {
    // send syscalls, interrupts, and exceptions to uservec in trampoline.S
    let trampoline_uservec = TRAMPOLINE + (uservec.as_ptr() as u64 - trampoline.as_ptr() as u64);
    w_stvec(trampoline_uservec);

    // set up trapframe values that uservec will need when
    // the process next traps into the kernel.
    let tf = (*p).trapframe;
    (*tf).kernel_satp = r_satp();                  // kernel page table
    (*tf).kernel_sp = (*p).kstack + KSTACKSIZE;    // process's kernel stack
    (*tf).kernel_trap = usertrap as *const () as u64;
    (*tf).kernel_hartid = r_tp();                  // hartid for cpuid()

    // set up the registers that trampoline.S's sret will use
    // to get to user space.

    // set S Previous Privilege mode to User.
    let mut x = r_sstatus();
    x &= !SSTATUS_SPP; // clear SPP to 0 for user mode
    x |= SSTATUS_SPIE; // enable interrupts in user mode
    w_sstatus(x);

    // set S Exception Program Counter to the saved user pc.
    w_sepc((*tf).epc);
  }
}

// interrupts and exceptions from kernel code go here via kernelvec,
// on whatever the current kernel stack is.
#[no_mangle]
pub extern "C" fn kerneltrap() {
  let sepc = r_sepc();
  let sstatus = r_sstatus();
  let scause = r_scause();

  if sstatus & SSTATUS_SPP == 0 {
    panic("kerneltrap: not from supervisor mode");
  }
  if intr_get() {
    panic("kerneltrap: interrupts enabled");
  }

  let which_dev = devintr();
  if which_dev == 0 {
    // interrupt or trap from an unknown source
    printf!("scause={:#x} sepc={:#x} stval={:#x}\n", scause, r_sepc(), r_stval());
    panic("kerneltrap");
  }

  // give up the CPU if this is a timer interrupt.
  if which_dev == 2 && !myproc().is_null() {
    yieldcpu();
  }

  // the yield() may have caused some traps to occur,
  // so restore trap registers for use by kernelvec.S's sepc instruction.
  w_sepc(sepc);
  w_sstatus(sstatus);
}

fn clockintr() {
  if cpuid() == 0 {
    unsafe {
      acquire(&raw mut TICKSLOCK);
      TICKS = TICKS.wrapping_add(1);
      wakeup(&raw const TICKS as *const u8);
      release(&raw mut TICKSLOCK);
    }
  }

  // ask for the next timer interrupt. this also clears
  // the interrupt request. 1000000 is about a tenth
  // of a second.
  w_stimecmp(r_time() + 1000000);
}

// check if it's an external interrupt or software interrupt,
// and handle it.
// returns 2 if timer interrupt,
// 1 if other device,
// 0 if not recognized.
fn devintr() -> i32 {
  let scause = r_scause();

  if scause == 0x8000000000000009 {
    // this is a supervisor external interrupt, via PLIC.

    // irq indicates which device interrupted.
    let irq = plic_claim();

    if irq == UART0_IRQ {
      uartintr();
    } else if irq == VIRTIO0_IRQ {
      virtio_disk_intr();
    } else if irq != 0 {
      printf!("unexpected interrupt irq={}\n", irq);
    }

    // the PLIC allows each device to raise at most one
    // interrupt at a time; tell the PLIC the device is
    // now allowed to interrupt again.
    if irq != 0 {
      plic_complete(irq);
    }

    1
  } else if scause == 0x8000000000000005 {
    // timer interrupt.
    clockintr();
    2
  } else {
    0
  }
}
