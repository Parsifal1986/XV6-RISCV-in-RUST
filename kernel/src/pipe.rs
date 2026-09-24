use crate::file::{filealloc, fileclose, File, FileType};
use crate::kalloc::{kalloc, kfree};
use crate::proc::{killed, myproc, sleep, wakeup};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::vm::{copyin, copyout};

pub const PIPESIZE: usize = 512;

pub struct Pipe {
  lock: Spinlock,
  data: [u8; PIPESIZE],
  nread: u32,      // number of bytes read
  nwrite: u32,     // number of bytes written
  readopen: bool,  // read fd is still open
  writeopen: bool, // write fd is still open
}

pub fn pipealloc(f0: &mut *mut File, f1: &mut *mut File) -> i32 {
  *f0 = filealloc();
  *f1 = if f0.is_null() { core::ptr::null_mut() } else { filealloc() };
  let pi = if f1.is_null() { core::ptr::null_mut() } else { kalloc() as *mut Pipe };

  if pi.is_null() {
    if !f0.is_null() {
      fileclose(*f0);
    }
    if !f1.is_null() {
      fileclose(*f1);
    }
    return -1;
  }

  unsafe {
    (*pi).readopen = true;
    (*pi).writeopen = true;
    (*pi).nwrite = 0;
    (*pi).nread = 0;
    initlock(&raw mut (*pi).lock, "pipe");
    (**f0).typ = FileType::FD_PIPE;
    (**f0).readable = true;
    (**f0).writable = false;
    (**f0).pipe = pi;
    (**f1).typ = FileType::FD_PIPE;
    (**f1).readable = false;
    (**f1).writable = true;
    (**f1).pipe = pi;
  }
  0
}

pub fn pipeclose(pi: *mut Pipe, writable: bool) {
  unsafe {
    acquire(&raw mut (*pi).lock);
    if writable {
      (*pi).writeopen = false;
      wakeup(&raw const (*pi).nread as *const u8);
    } else {
      (*pi).readopen = false;
      wakeup(&raw const (*pi).nwrite as *const u8);
    }
    if !(*pi).readopen && !(*pi).writeopen {
      release(&raw mut (*pi).lock);
      kfree(pi as *mut u8);
    } else {
      release(&raw mut (*pi).lock);
    }
  }
}

pub fn pipewrite(pi: *mut Pipe, addr: u64, n: i32) -> i32 {
  let mut i = 0;
  let pr = myproc();

  unsafe {
    acquire(&raw mut (*pi).lock);
    while i < n {
      if !(*pi).readopen || killed(pr) {
        release(&raw mut (*pi).lock);
        return -1;
      }
      if (*pi).nwrite == (*pi).nread.wrapping_add(PIPESIZE as u32) {
        //DOC: pipewrite-full
        wakeup(&raw const (*pi).nread as *const u8);
        sleep(&raw const (*pi).nwrite as *const u8, &raw mut (*pi).lock);
      } else {
        let mut ch = 0u8;
        if copyin((*pr).pagetable, &mut ch, addr + i as u64, 1) == -1 {
          if i == 0 {
            i = -1;
          }
          break;
        }
        (*pi).data[(*pi).nwrite as usize % PIPESIZE] = ch;
        (*pi).nwrite = (*pi).nwrite.wrapping_add(1);
        i += 1;
      }
    }
    wakeup(&raw const (*pi).nread as *const u8);
    release(&raw mut (*pi).lock);
  }

  i
}

pub fn piperead(pi: *mut Pipe, addr: u64, n: i32) -> i32 {
  let pr = myproc();

  unsafe {
    acquire(&raw mut (*pi).lock);
    while (*pi).nread == (*pi).nwrite && (*pi).writeopen {
      //DOC: pipe-empty
      if killed(pr) {
        release(&raw mut (*pi).lock);
        return -1;
      }
      sleep(&raw const (*pi).nread as *const u8, &raw mut (*pi).lock); //DOC: piperead-sleep
    }
    let mut i = 0;
    while i < n {
      //DOC: piperead-copy
      if (*pi).nread == (*pi).nwrite {
        break;
      }
      let ch = (*pi).data[(*pi).nread as usize % PIPESIZE];
      if copyout((*pr).pagetable, addr + i as u64, &ch, 1) == -1 {
        if i == 0 {
          i = -1;
        }
        break;
      }
      (*pi).nread = (*pi).nread.wrapping_add(1);
      i += 1;
    }
    wakeup(&raw const (*pi).nwrite as *const u8); //DOC: piperead-wakeup
    release(&raw mut (*pi).lock);
    i
  }
}
