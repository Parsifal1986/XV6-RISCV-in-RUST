use crate::printf::panic;
use crate::proc::myproc;
use crate::string::{cstr, strlen};
use crate::sysfile::{
  sys_chdir, sys_close, sys_dup, sys_exec, sys_fstat, sys_link, sys_mkdir, sys_mknod, sys_open, sys_pipe, sys_read,
  sys_unlink, sys_write,
};
use crate::sysproc::{sys_exit, sys_fork, sys_getpid, sys_kill, sys_pause, sys_sbrk, sys_uptime, sys_wait};
use crate::vm::{copyin, copyinstr};

// System call numbers
pub const SYS_fork: usize = 1;
pub const SYS_exit: usize = 2;
pub const SYS_wait: usize = 3;
pub const SYS_pipe: usize = 4;
pub const SYS_read: usize = 5;
pub const SYS_kill: usize = 6;
pub const SYS_exec: usize = 7;
pub const SYS_fstat: usize = 8;
pub const SYS_chdir: usize = 9;
pub const SYS_dup: usize = 10;
pub const SYS_getpid: usize = 11;
pub const SYS_sbrk: usize = 12;
pub const SYS_pause: usize = 13;
pub const SYS_uptime: usize = 14;
pub const SYS_open: usize = 15;
pub const SYS_write: usize = 16;
pub const SYS_mknod: usize = 17;
pub const SYS_unlink: usize = 18;
pub const SYS_link: usize = 19;
pub const SYS_mkdir: usize = 20;
pub const SYS_close: usize = 21;

// Fetch the u64 at addr from the current process.
pub fn fetchaddr(addr: u64, ip: &mut u64) -> i32 {
  let p = myproc();
  unsafe {
    // both tests needed, in case of overflow
    if addr >= (*p).sz || addr.wrapping_add(size_of::<u64>() as u64) > (*p).sz {
      return -1;
    }
    if copyin((*p).pagetable, ip as *mut u64 as *mut u8, addr, size_of::<u64>() as u64) != 0 {
      return -1;
    }
  }
  0
}

// Fetch the nul-terminated string at addr from the current process.
// Returns length of string, not including nul, or -1 for error.
pub fn fetchstr(addr: u64, buf: &mut [u8]) -> i32 {
  let p = myproc();
  unsafe {
    if copyinstr((*p).pagetable, buf.as_mut_ptr(), addr, buf.len() as u64) < 0 {
      return -1;
    }
  }
  strlen(buf.as_ptr()) as i32
}

fn argraw(n: i32) -> u64 {
  let p = myproc();
  unsafe {
    let tf = (*p).trapframe;
    match n {
      0 => (*tf).a0,
      1 => (*tf).a1,
      2 => (*tf).a2,
      3 => (*tf).a3,
      4 => (*tf).a4,
      5 => (*tf).a5,
      _ => panic("argraw"),
    }
  }
}

// Fetch the nth 32-bit system call argument.
pub fn argint(n: i32, ip: &mut i32) {
  *ip = argraw(n) as i32;
}

// Retrieve an argument as a pointer.
// Doesn't check for legality, since
// copyin/copyout will do that.
pub fn argaddr(n: i32, ip: &mut u64) {
  *ip = argraw(n);
}

// Fetch the nth word-sized system call argument as a null-terminated string.
// Copies into buf, at most buf.len().
// Returns string length if OK (not including nul), -1 if error.
pub fn argstr(n: i32, buf: &mut [u8]) -> i32 {
  let mut addr = 0;
  argaddr(n, &mut addr);
  fetchstr(addr, buf)
}

// the NUL-terminated string in buf, as fetched by argstr().
pub fn argbuf(buf: &[u8]) -> &[u8] {
  cstr(buf)
}

type SysCallFn = fn() -> u64;

// An array mapping syscall numbers
// to the function that handles the system call.
static SYSCALLS: [Option<SysCallFn>; 22] = [
  None,
  Some(sys_fork),
  Some(sys_exit),
  Some(sys_wait),
  Some(sys_pipe),
  Some(sys_read),
  Some(sys_kill),
  Some(sys_exec),
  Some(sys_fstat),
  Some(sys_chdir),
  Some(sys_dup),
  Some(sys_getpid),
  Some(sys_sbrk),
  Some(sys_pause),
  Some(sys_uptime),
  Some(sys_open),
  Some(sys_write),
  Some(sys_mknod),
  Some(sys_unlink),
  Some(sys_link),
  Some(sys_mkdir),
  Some(sys_close),
];

pub fn syscall() {
  let p = myproc();
  unsafe {
    let num = (*(*p).trapframe).a7 as usize;
    match SYSCALLS.get(num) {
      Some(Some(f)) => {
        // Use num to lookup the system call function for num, call it,
        // and store its return value in p->trapframe->a0
        (*(*p).trapframe).a0 = f();
      }
      _ => {
        let name = core::str::from_utf8(cstr(&(*p).name)).unwrap_or("???");
        printf!("{} {}: unknown sys call {}\n", (*p).pid, name, num);
        (*(*p).trapframe).a0 = u64::MAX;
      }
    }
  }
}
