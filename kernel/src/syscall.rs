use crate::printf::{printf, panic};
use crate::proc::myproc;
use crate::sysproc::{sys_fork, sys_kill, sys_uptime, sys_wait, sys_exit, sys_getpid, sys_sbrk, sys_pause};
use crate::sysfile::{sys_dup, sys_open, sys_read, sys_write, sys_close, sys_fstat, sys_pipe, sys_chdir, sys_exec, sys_mknod, sys_unlink, sys_link, sys_mkdir};
use crate::vm::{copyin, copyinstr};

pub const SYS_FORK: usize = 1;
pub const SYS_EXIT: usize = 2;
pub const SYS_WAIT: usize = 3;
pub const SYS_PIPE: usize = 4;
pub const SYS_READ: usize = 5;
pub const SYS_KILL: usize = 6;
pub const SYS_EXEC: usize = 7;
pub const SYS_FSTAT: usize = 8;
pub const SYS_CHDIR: usize = 9;
pub const SYS_DUP: usize = 10;
pub const SYS_GETPID: usize = 11;
pub const SYS_SBRK: usize = 12;
pub const SYS_PAUSE: usize = 13;
pub const SYS_UPTIME: usize = 14;
pub const SYS_OPEN: usize = 15;
pub const SYS_WRITE: usize = 16;
pub const SYS_MKNOD: usize = 17;
pub const SYS_UNLINK: usize = 18;
pub const SYS_LINK: usize = 19;
pub const SYS_MKDIR: usize = 20;
pub const SYS_CLOSE: usize = 21;

pub fn fetchaddr(addr: u64, ip: *mut u64) -> i32 {
  let p = myproc().unwrap();

  if addr >= p.sz || addr + size_of::<u64>() as u64 > p.sz {
    return -1;
  }
  if copyin(p.pagetable, ip as u64, addr, size_of::<u64>() as u64) != 0 {
    return -1;
  }
  0
}

pub fn fetchstr(addr: u64, buf: &mut [u8], max: i32) -> i32 {
  let p = myproc().unwrap();
  if copyinstr(p.pagetable, buf.as_mut_ptr() as u64, addr, max as u64) < 0 {
    return -1;
  }
  buf.len() as i32
}

fn argraw(n: i32) -> u64 {
  let p = myproc().unwrap();
  match n {
    0 => unsafe { (*p.trapframe).a0 },
    1 => unsafe { (*p.trapframe).a1 },
    2 => unsafe { (*p.trapframe).a2 },
    3 => unsafe { (*p.trapframe).a3 },
    4 => unsafe { (*p.trapframe).a4 },
    5 => unsafe { (*p.trapframe).a5 },
    _ => panic("argraw"),
  }
}

pub fn argint(n: i32, ip: &mut i32) -> i32 {
  *ip = argraw(n) as i32;
  0
}

pub fn argaddr(n: i32, ip: &mut u64) -> i32 {
  *ip = argraw(n);
  0
}

pub fn argstr(n: i32, buf: &mut [u8], max: i32) -> i32 {
  let addr = argraw(n);
  fetchstr(addr, buf, max)
}

type SysCallFn = fn() -> u64;

fn emptyfn() -> u64 {
  0
}

static SYSCALLS: [SysCallFn; 22] = [
  emptyfn,
  sys_fork,
  sys_exit,
  sys_wait,
  sys_pipe,
  sys_read,
  sys_kill,
  sys_exec,
  sys_fstat,
  sys_chdir,
  sys_dup,
  sys_getpid,
  sys_sbrk,
  sys_pause,
  sys_uptime,
  sys_open,
  sys_write,
  sys_mknod,
  sys_unlink,
  sys_link,
  sys_mkdir,
  sys_close,
];

pub fn syscall() {
  let p = myproc().unwrap();
  let num = unsafe { (*p.trapframe).a7 };

  if num > 0 && (num as usize) < SYSCALLS.len() && !core::ptr::fn_addr_eq(SYSCALLS[num as usize], emptyfn as fn() -> u64) {
    unsafe {
      (*p.trapframe).a0 = SYSCALLS[num as usize]();
    }
  } else {
    unsafe {
      printf(format_args!("{} {}: unknown sys call {}\n", p.pid, core::str::from_utf8_unchecked(&p.name), num));
      (*p.trapframe).a0 = u64::MAX;
    }
  }
}