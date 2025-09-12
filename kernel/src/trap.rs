use crate::memlayout::{TRAMPOLINE, UART0_IRQ, VIRTIO0, VIRTIO0_IRQ};
use crate::plic::{plic_claim, plic_complete};
use crate::syscall::syscall;
use crate::printf::{panic, printf};
use crate::proc::{cpuid, kexit, killed, myproc, setkilled, wakeup, yieldcpu};
use crate::riscv::{intr_get, intr_off, intr_on, r_satp, r_scause, r_sepc, r_sstatus, r_stval, r_time, r_tp, w_sepc, w_sstatus, w_stimecmp, w_stvec, MAKE_SATP, PGSIZE, SSTATUS_SPIE, SSTATUS_SPP};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::uart::uartinit;
use crate::vm::vmfault;

pub static mut tickslock: Spinlock = Spinlock::new();
pub static mut ticks: u32 = 0;

extern "C" {
  fn tramponline();
  fn uservec();
  fn kernelvec();
}

pub fn trapinit() {
  initlock(unsafe { &mut tickslock }, Some("time".as_bytes()));
}

pub fn trapinithart() {
  w_stvec(kernelvec as u64);
}

pub fn usertrap() -> u64 {
  let mut which_dev = 0;

  if r_sstatus() & SSTATUS_SPP != 0 {
    panic("usertrap: not from user mode");
  }

  w_stvec(kernelvec as u64);

  let p = myproc().unwrap();
  unsafe {
    (*p.trapframe).epc = r_sepc();
  }

  if r_scause() == 8 {
    if killed(p) {
      kexit(-1);
    }

    unsafe {
      (*p.trapframe).epc += 4;
    }

    intr_on();

    syscall();
  } else if {which_dev = devintr(); which_dev} != 0 {

  } else if (r_scause() == 15 || r_scause() == 13) && vmfault(p.pagetable, r_stval(), (if r_scause() == 13 { 1 } else { 0 } != 0) as i32) != 0 {
  } else {
    printf(format_args!("usertrap(): unexpected scause {} pid={}\n", r_scause(), p.pid as i32));
    printf(format_args!("            sepc={} stval={}\n", r_sepc(), r_stval()));
    setkilled(p);
  }

  if killed(p) {
    kexit(-1);
  }

  if which_dev == 2 {
    yieldcpu();
  }

  prepare_return();

  let satp = MAKE_SATP(p.pagetable as u64);

  return satp
}

pub fn prepare_return() {
  let p = myproc().unwrap();

  intr_off();

  let tramponline_uervec = TRAMPOLINE + unsafe { (uservec as u64) - (tramponline as u64) };
  w_stvec(tramponline_uervec);

  unsafe {
    (*p.trapframe).kernel_satp = r_satp();
    (*p.trapframe).kernel_sp = p.kstack + PGSIZE;
    (*p.trapframe).kernel_trap = usertrap as u64;
    (*p.trapframe).kernel_hartid = r_tp();
  }

  let x = (r_sstatus() & !SSTATUS_SPP) | SSTATUS_SPIE;
  w_sstatus(x);

  w_sepc(unsafe { (*p.trapframe).epc });
}

fn kerneltrap() {
  let which_dev;
  let sepc = r_sepc();
  let sstatus = r_sstatus();
  let scause = r_scause();

  if sstatus & SSTATUS_SPP == 0 {
    panic("kerneltrap: not from supervisor mode");
  }
  if intr_get() != 0 {
    panic("kerneltrap: interrupts enabled");
  }

  if {which_dev = devintr(); which_dev} == 0 {
    printf(format_args!("scause {} sepc {} stval {}\n", scause, sepc, r_stval()));
    panic("kerneltrap");
  }

  if which_dev == 2 && myproc().is_some() {
    yieldcpu();
  }

  w_sepc(sepc);
  w_sstatus(sstatus);
}

fn clockintr() {
  if cpuid() == 0 {
    unsafe {
      acquire(&mut tickslock);
      ticks += 1;
      wakeup(&mut ticks as *mut u32 as *mut u8);
      release(&mut tickslock);
    }
  }
  w_stimecmp(r_time() + 100000);
}

fn devintr() -> i32 {
  let scause = r_scause();

  if scause == 0x8000000000000009 {
    let irq = plic_claim();

    if irq == UART0_IRQ as i32 {
      uartinit();
    }  else if irq == VIRTIO0_IRQ as i32 {
      // virtio_disk_intr();
    } else if irq != 0 {
      printf(format_args!("unexpected interrupt irq={}\n", irq));
    }

    if irq != 0 {
      plic_complete(irq);
    }
    return 1;
  } else if scause == 0x8000000000000005 {
    clockintr();
    return 2;
  } else {
    return 0;
  }
}