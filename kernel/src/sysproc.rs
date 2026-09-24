use crate::proc::{growproc, kexit, kfork, kkill, killed, kwait, myproc, sleep};
use crate::spinlock::{acquire, release};
use crate::syscall::{argaddr, argint};
use crate::trap::{TICKS, TICKSLOCK};
use crate::vm::SBRK_EAGER;

pub fn sys_exit() -> u64 {
  let mut n: i32 = 0;
  argint(0, &mut n);
  kexit(n);
}

pub fn sys_getpid() -> u64 {
  unsafe { (*myproc()).pid as u64 }
}

pub fn sys_fork() -> u64 {
  kfork() as u64
}

pub fn sys_wait() -> u64 {
  let mut p: u64 = 0;
  argaddr(0, &mut p);
  kwait(p) as u64
}

pub fn sys_sbrk() -> u64 {
  let mut n: i32 = 0;
  let mut t: i32 = 0;

  argint(0, &mut n);
  argint(1, &mut t);
  let p = myproc();
  let addr = unsafe { (*p).sz };

  if t == SBRK_EAGER || n < 0 {
    if growproc(n) < 0 {
      return u64::MAX;
    }
  } else {
    // Lazily allocate memory for this process: increase its memory
    // size but don't allocate memory. If the processes uses the
    // memory, vmfault() will allocate it.
    let newsz = addr.wrapping_add(n as u64);
    if newsz < addr {
      return u64::MAX;
    }
    unsafe {
      (*p).sz = newsz;
    }
  }
  addr
}

pub fn sys_pause() -> u64 {
  let mut n: i32 = 0;

  argint(0, &mut n);
  if n < 0 {
    n = 0;
  }
  unsafe {
    acquire(&raw mut TICKSLOCK);
    let ticks0 = TICKS;
    while core::ptr::read_volatile(&raw const TICKS).wrapping_sub(ticks0) < n as u32 {
      if killed(myproc()) {
        release(&raw mut TICKSLOCK);
        return u64::MAX;
      }
      sleep(&raw const TICKS as *const u8, &raw mut TICKSLOCK);
    }
    release(&raw mut TICKSLOCK);
  }
  0
}

pub fn sys_kill() -> u64 {
  let mut pid: i32 = 0;

  argint(0, &mut pid);
  kkill(pid) as u64
}

// return how many clock tick interrupts have occurred
// since start.
pub fn sys_uptime() -> u64 {
  unsafe {
    acquire(&raw mut TICKSLOCK);
    let xticks = TICKS;
    release(&raw mut TICKSLOCK);
    xticks as u64
  }
}
