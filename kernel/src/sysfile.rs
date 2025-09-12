use crate::{file::{filedup, File, Inode}, param::NOFILE, proc::myproc, syscall::argint};

fn argfd(n: i32, pdf: Option<&mut i32>, pf: Option<&mut *mut File>) -> i32 {
  let mut fd: i32 = 0;
  
  argint(n, &mut fd);
  let f = myproc().unwrap().ofile[fd as usize];
  if fd < 0 || fd > NOFILE as i32 || f.is_none() {
    return -1;
  }
  if let Some(pdf) = pdf {
    *pdf = fd;
  }
  if let Some(pf) = pf {
    *pf = f.unwrap();
  }
  
  return 0;
}

fn fdalloc(f: *mut File) -> i32 {
  let p = myproc().unwrap();
  for fd in 0..NOFILE as i32 {
    if p.ofile[fd as usize].is_none() {
      p.ofile[fd as usize] = Some(f);
      return fd;
    }
  }
  -1
}

pub fn sys_dup() -> u64 {
  let mut f: *mut File = core::ptr::null_mut();
  if argfd(0, None, Some(&mut f)) < 0 {
    return u64::MAX;
  }
  let fd = fdalloc(f);
  if fd < 0 {
    return u64::MAX;
  }
  filedup(f);
  fd as u64
}

pub fn sys_read() -> u64 {
  todo!()
}

pub fn sys_write() -> u64 {
  todo!()
}

pub fn sys_close() -> u64 {
  todo!()
}

pub fn sys_fstat() -> u64 {
  todo!()
}

pub fn sys_link() -> u64 {
  todo!()
}

pub fn isdirempty(dp: *mut Inode) -> i32 {
  todo!()
}

pub fn sys_unlink() -> u64 {
  todo!()
}

pub fn sys_open() -> u64 {
  todo!()
}

pub fn sys_mkdir() -> u64 {
  todo!()
}

pub fn sys_mknod() -> u64 {
  todo!()
}

pub fn sys_chdir() -> u64 {
  todo!()
}

pub fn sys_exec() -> u64 {
  todo!()
}

pub fn sys_pipe() -> u64 {
  todo!()
}