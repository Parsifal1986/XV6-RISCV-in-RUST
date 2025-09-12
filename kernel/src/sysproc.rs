use crate::proc::{growproc, kexit, kfork, killed, kkill, kwait, myproc, sleep};
use crate::spinlock::{acquire, release};
use crate::syscall::{argaddr, argint};
use crate::trap::{ticks, tickslock};
use crate::vm::SBRK_EAGER;

pub fn sys_exit() -> u64 {
  let mut n: i32 = 0;
  argint(0, &mut n);
  kexit(n);
  0
}

pub fn sys_getpid() -> u64 {
  myproc().unwrap().pid as u64
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
  let (mut t, mut n) : (i32, i32) = (0, 0);
  let addr = myproc().unwrap().sz;

  argint(0, &mut n);
  argint(1, &mut t);

  if t == SBRK_EAGER || n < 0 {
    if growproc(n) < 0 {
      return u64::MAX;
    }
  } else {
    if addr.wrapping_add(n as u64) < addr {
      return u64::MAX;
    }
    myproc().unwrap().sz = addr.wrapping_add(n as u64);
  }
  addr
}

pub fn sys_pause() -> u64 {
  let mut n: i32 = 0;
  let ticks0: u32;

  argint(0, &mut n);
  if n < 0 {
    return u64::MAX;
  }

  unsafe {
    acquire(&mut tickslock);
    ticks0 = ticks;
    while ticks - ticks0 < n as u32 {
      if killed(myproc().unwrap()) {
        release(&mut tickslock);
        return u64::MAX;
      }
      sleep(&mut ticks as *mut u32 as *mut u8, &mut tickslock);
    }
    release(&mut tickslock);
  }
  0
}

pub fn sys_kill() -> u64 {
  let mut pid: i32 = 0;
  
  argint(0, &mut pid);
  kkill(pid) as u64
}

pub fn sys_uptime() -> u64 {
  let xticks: u32;

  unsafe {
    acquire(&mut tickslock);
    xticks = ticks;
    release(&mut tickslock);
  }
  xticks as u64
}