// System call wrappers that take Rust types, and
// some string helpers.

use alloc::vec::Vec;

use crate::syscall::raw;
use crate::{Stat, O_RDONLY, SBRK_EAGER, SBRK_LAZY};

// length of a NUL-terminated string.
pub fn strlen_ptr(p: *const u8) -> usize {
  let mut n = 0;
  unsafe {
    while *p.add(n) != 0 {
      n += 1;
    }
  }
  n
}

// the part of buf before the first NUL (or all of buf).
pub fn cstr(buf: &[u8]) -> &[u8] {
  match buf.iter().position(|&c| c == 0) {
    Some(n) => &buf[..n],
    None => buf,
  }
}

// buf up to the first NUL, as a str.
pub fn as_str(buf: &[u8]) -> &str {
  let s = cstr(buf);
  match core::str::from_utf8(s) {
    Ok(s) => s,
    Err(e) => unsafe { core::str::from_utf8_unchecked(&s[..e.valid_up_to()]) },
  }
}

// call f with a NUL-terminated copy of s.
pub fn with_cstr<R>(s: &[u8], f: impl FnOnce(*const u8) -> R) -> R {
  let s = cstr(s);
  let mut buf = [0u8; 256];
  if s.len() < buf.len() {
    buf[..s.len()].copy_from_slice(s);
    buf[s.len()] = 0;
    f(buf.as_ptr())
  } else {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s);
    v.push(0);
    f(v.as_ptr())
  }
}

pub fn fork() -> i32 {
  raw::fork()
}

pub fn exit(status: i32) -> ! {
  raw::exit(status)
}

// wait for a child to exit; returns its pid, or -1.
// if status is Some, the child's exit status is stored there.
pub fn wait(status: Option<&mut i32>) -> i32 {
  match status {
    Some(s) => raw::wait(s),
    None => raw::wait(core::ptr::null_mut()),
  }
}

pub fn pipe(fds: &mut [i32; 2]) -> i32 {
  raw::pipe(fds.as_mut_ptr())
}

pub fn read(fd: i32, buf: &mut [u8]) -> i32 {
  raw::read(fd, buf.as_mut_ptr(), buf.len() as i32)
}

pub fn write(fd: i32, buf: &[u8]) -> i32 {
  raw::write(fd, buf.as_ptr(), buf.len() as i32)
}

pub fn close(fd: i32) -> i32 {
  raw::close(fd)
}

pub fn kill(pid: i32) -> i32 {
  raw::kill(pid)
}

pub fn exec<P: AsRef<[u8]> + ?Sized, A: AsRef<[u8]>>(path: &P, argv: &[A]) -> i32 {
  let args: Vec<Vec<u8>> = argv
    .iter()
    .map(|a| {
      let mut v = Vec::from(cstr(a.as_ref()));
      v.push(0);
      v
    })
    .collect();
  let mut ptrs: Vec<*const u8> = args.iter().map(|a| a.as_ptr()).collect();
  ptrs.push(core::ptr::null());
  with_cstr(path.as_ref(), |p| raw::exec(p, ptrs.as_ptr()))
}

pub fn open<P: AsRef<[u8]> + ?Sized>(path: &P, omode: i32) -> i32 {
  with_cstr(path.as_ref(), |p| raw::open(p, omode))
}

pub fn mknod<P: AsRef<[u8]> + ?Sized>(path: &P, major: i16, minor: i16) -> i32 {
  with_cstr(path.as_ref(), |p| raw::mknod(p, major, minor))
}

pub fn unlink<P: AsRef<[u8]> + ?Sized>(path: &P) -> i32 {
  with_cstr(path.as_ref(), |p| raw::unlink(p))
}

pub fn fstat(fd: i32, st: &mut Stat) -> i32 {
  raw::fstat(fd, st)
}

pub fn link<P: AsRef<[u8]> + ?Sized, Q: AsRef<[u8]> + ?Sized>(old: &P, new: &Q) -> i32 {
  with_cstr(old.as_ref(), |o| with_cstr(new.as_ref(), |n| raw::link(o, n)))
}

pub fn mkdir<P: AsRef<[u8]> + ?Sized>(path: &P) -> i32 {
  with_cstr(path.as_ref(), |p| raw::mkdir(p))
}

pub fn chdir<P: AsRef<[u8]> + ?Sized>(path: &P) -> i32 {
  with_cstr(path.as_ref(), |p| raw::chdir(p))
}

pub fn dup(fd: i32) -> i32 {
  raw::dup(fd)
}

pub fn getpid() -> i32 {
  raw::getpid()
}

// grow the heap by n bytes, allocating memory now.
pub fn sbrk(n: i32) -> *mut u8 {
  raw::sbrk(n, SBRK_EAGER)
}

// grow the heap by n bytes, allocating memory on first use.
pub fn sbrklazy(n: i32) -> *mut u8 {
  raw::sbrk(n, SBRK_LAZY)
}

pub fn pause(n: i32) -> i32 {
  raw::pause(n)
}

pub fn uptime() -> i32 {
  raw::uptime()
}

pub fn stat<P: AsRef<[u8]> + ?Sized>(path: &P, st: &mut Stat) -> i32 {
  let fd = open(path, O_RDONLY);
  if fd < 0 {
    return -1;
  }
  let r = fstat(fd, st);
  close(fd);
  r
}

// read a line from stdin into buf, including the newline.
// returns the number of bytes read; 0 means end of file.
pub fn gets(buf: &mut [u8]) -> usize {
  let mut i = 0;
  while i + 1 < buf.len() {
    let mut c = [0u8; 1];
    let cc = read(0, &mut c);
    if cc < 1 {
      break;
    }
    buf[i] = c[0];
    i += 1;
    if c[0] == b'\n' || c[0] == b'\r' {
      break;
    }
  }
  buf[i] = 0;
  i
}

pub fn atoi<S: AsRef<[u8]> + ?Sized>(s: &S) -> i32 {
  let mut n: i32 = 0;
  for &c in s.as_ref() {
    if !c.is_ascii_digit() {
      break;
    }
    n = n.wrapping_mul(10).wrapping_add((c - b'0') as i32);
  }
  n
}
