// System call stubs.  Each puts its arguments in a0..a5,
// the system call number in a7, and executes ecall.
// The kernel returns the result in a0.

use core::arch::asm;

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

#[inline(always)]
pub fn syscall(num: usize, a0: usize, a1: usize, a2: usize) -> isize {
  let ret: isize;
  unsafe {
    asm!(
      "ecall",
      inlateout("a0") a0 => ret,
      in("a1") a1,
      in("a2") a2,
      in("a7") num,
      options(nostack)
    );
  }
  ret
}

// Raw system calls, taking pointers exactly as the kernel sees them.
// Strings must be NUL-terminated.  These are useful for programs
// (like usertests) that deliberately pass bad arguments.
pub mod raw {
  use super::*;

  pub fn fork() -> i32 {
    syscall(SYS_FORK, 0, 0, 0) as i32
  }

  pub fn exit(status: i32) -> ! {
    syscall(SYS_EXIT, status as usize, 0, 0);
    unreachable!()
  }

  pub fn wait(status: *mut i32) -> i32 {
    syscall(SYS_WAIT, status as usize, 0, 0) as i32
  }

  pub fn pipe(fds: *mut i32) -> i32 {
    syscall(SYS_PIPE, fds as usize, 0, 0) as i32
  }

  pub fn read(fd: i32, buf: *mut u8, n: i32) -> i32 {
    syscall(SYS_READ, fd as usize, buf as usize, n as usize) as i32
  }

  pub fn write(fd: i32, buf: *const u8, n: i32) -> i32 {
    syscall(SYS_WRITE, fd as usize, buf as usize, n as usize) as i32
  }

  pub fn close(fd: i32) -> i32 {
    syscall(SYS_CLOSE, fd as usize, 0, 0) as i32
  }

  pub fn kill(pid: i32) -> i32 {
    syscall(SYS_KILL, pid as usize, 0, 0) as i32
  }

  pub fn exec(path: *const u8, argv: *const *const u8) -> i32 {
    syscall(SYS_EXEC, path as usize, argv as usize, 0) as i32
  }

  pub fn open(path: *const u8, omode: i32) -> i32 {
    syscall(SYS_OPEN, path as usize, omode as usize, 0) as i32
  }

  pub fn mknod(path: *const u8, major: i16, minor: i16) -> i32 {
    syscall(SYS_MKNOD, path as usize, major as usize, minor as usize) as i32
  }

  pub fn unlink(path: *const u8) -> i32 {
    syscall(SYS_UNLINK, path as usize, 0, 0) as i32
  }

  pub fn fstat(fd: i32, st: *mut crate::Stat) -> i32 {
    syscall(SYS_FSTAT, fd as usize, st as usize, 0) as i32
  }

  pub fn link(old: *const u8, new: *const u8) -> i32 {
    syscall(SYS_LINK, old as usize, new as usize, 0) as i32
  }

  pub fn mkdir(path: *const u8) -> i32 {
    syscall(SYS_MKDIR, path as usize, 0, 0) as i32
  }

  pub fn chdir(path: *const u8) -> i32 {
    syscall(SYS_CHDIR, path as usize, 0, 0) as i32
  }

  pub fn dup(fd: i32) -> i32 {
    syscall(SYS_DUP, fd as usize, 0, 0) as i32
  }

  pub fn getpid() -> i32 {
    syscall(SYS_GETPID, 0, 0, 0) as i32
  }

  pub fn sbrk(n: i32, t: i32) -> *mut u8 {
    syscall(SYS_SBRK, n as usize, t as usize, 0) as *mut u8
  }

  pub fn pause(n: i32) -> i32 {
    syscall(SYS_PAUSE, n as usize, 0, 0) as i32
  }

  pub fn uptime() -> i32 {
    syscall(SYS_UPTIME, 0, 0, 0) as i32
  }
}
