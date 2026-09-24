//
// Support functions for system calls that involve file descriptors.
//

use core::ptr::null_mut;

use crate::fs::{ilock, iput, iunlock, readi, stati, writei, BSIZE, NDIRECT};
use crate::log::{begin_op, end_op};
use crate::param::{MAXOPBLOCKS, NDEV, NFILE};
use crate::pipe::{pipeclose, piperead, pipewrite, Pipe};
use crate::printf::panic;
use crate::proc::myproc;
use crate::sleeplock::Sleeplock;
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::stat::Stat;
use crate::vm::copyout;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileType {
  FD_NONE,
  FD_PIPE,
  FD_INODE,
  FD_DEVICE,
}

#[derive(Clone, Copy)]
pub struct File {
  pub typ: FileType,
  pub r#ref: i32, // reference count
  pub readable: bool,
  pub writable: bool,
  pub pipe: *mut Pipe, // FD_PIPE
  pub ip: *mut Inode,  // FD_INODE and FD_DEVICE
  pub off: u32,        // FD_INODE
  pub major: i16,      // FD_DEVICE
}

impl File {
  pub const fn new() -> Self {
    File {
      typ: FileType::FD_NONE,
      r#ref: 0,
      readable: false,
      writable: false,
      pipe: null_mut(),
      ip: null_mut(),
      off: 0,
      major: 0,
    }
  }
}

// in-memory copy of an inode
pub struct Inode {
  pub dev: u32,           // Device number
  pub inum: u32,          // Inode number
  pub r#ref: i32,         // Reference count
  pub lock: Sleeplock,    // protects everything below here
  pub valid: bool,        // inode has been read from disk?

  pub typ: i16,           // copy of disk inode
  pub major: i16,
  pub minor: i16,
  pub nlink: i16,
  pub size: u32,
  pub addrs: [u32; NDIRECT + 1],
}

impl Inode {
  pub const fn new() -> Self {
    Inode {
      dev: 0,
      inum: 0,
      r#ref: 0,
      lock: Sleeplock::new(),
      valid: false,
      typ: 0,
      major: 0,
      minor: 0,
      nlink: 0,
      size: 0,
      addrs: [0; NDIRECT + 1],
    }
  }
}

// map major device number to device functions.
pub struct Devsw {
  pub read: Option<fn(bool, u64, i32) -> i32>,
  pub write: Option<fn(bool, u64, i32) -> i32>,
}

impl Devsw {
  pub const fn new() -> Self {
    Devsw { read: None, write: None }
  }
}

pub static mut DEVSW: [Devsw; NDEV] = [const { Devsw::new() }; NDEV];

pub const CONSOLE: usize = 1;

struct Ftable {
  lock: Spinlock,
  file: [File; NFILE],
}

static mut FTABLE: Ftable = Ftable {
  lock: Spinlock::new(),
  file: [const { File::new() }; NFILE],
};

pub fn fileinit() {
  unsafe {
    initlock(&raw mut FTABLE.lock, "ftable");
  }
}

// Allocate a file structure.
pub fn filealloc() -> *mut File {
  unsafe {
    acquire(&raw mut FTABLE.lock);
    for i in 0..NFILE {
      let f = &raw mut FTABLE.file[i];
      if (*f).r#ref == 0 {
        (*f).r#ref = 1;
        release(&raw mut FTABLE.lock);
        return f;
      }
    }
    release(&raw mut FTABLE.lock);
  }
  null_mut()
}

// Increment ref count for file f.
pub fn filedup(f: *mut File) -> *mut File {
  unsafe {
    acquire(&raw mut FTABLE.lock);
    if (*f).r#ref < 1 {
      panic("filedup");
    }
    (*f).r#ref += 1;
    release(&raw mut FTABLE.lock);
  }
  f
}

// Close file f.  (Decrement ref count, close when reaches 0.)
pub fn fileclose(f: *mut File) {
  let ff: File;

  unsafe {
    acquire(&raw mut FTABLE.lock);
    if (*f).r#ref < 1 {
      panic("fileclose");
    }
    (*f).r#ref -= 1;
    if (*f).r#ref > 0 {
      release(&raw mut FTABLE.lock);
      return;
    }
    ff = *f;
    (*f).r#ref = 0;
    (*f).typ = FileType::FD_NONE;
    release(&raw mut FTABLE.lock);
  }

  if ff.typ == FileType::FD_PIPE {
    pipeclose(ff.pipe, ff.writable);
  } else if ff.typ == FileType::FD_INODE || ff.typ == FileType::FD_DEVICE {
    begin_op();
    iput(ff.ip);
    end_op();
  }
}

// Get metadata about file f.
// addr is a user virtual address, pointing to a struct stat.
pub fn filestat(f: *mut File, addr: u64) -> i32 {
  let p = myproc();
  let mut st = Stat { dev: 0, ino: 0, typ: 0, nlink: 0, size: 0 };

  unsafe {
    if (*f).typ == FileType::FD_INODE || (*f).typ == FileType::FD_DEVICE {
      ilock((*f).ip);
      stati((*f).ip, &mut st);
      iunlock((*f).ip);
      if copyout((*p).pagetable, addr, &raw const st as *const u8, size_of::<Stat>() as u64) < 0 {
        return -1;
      }
      return 0;
    }
  }
  -1
}

// Read from file f.
// addr is a user virtual address.
pub fn fileread(f: *mut File, addr: u64, n: i32) -> i32 {
  unsafe {
    if !(*f).readable || n < 0 {
      return -1;
    }

    match (*f).typ {
      FileType::FD_PIPE => piperead((*f).pipe, addr, n),
      FileType::FD_DEVICE => {
        let major = (*f).major;
        if major < 0 || major as usize >= NDEV {
          return -1;
        }
        match DEVSW[major as usize].read {
          Some(read) => read(true, addr, n),
          None => -1,
        }
      }
      FileType::FD_INODE => {
        ilock((*f).ip);
        let r = readi((*f).ip, true, addr, (*f).off, n as u32);
        if r > 0 {
          (*f).off += r as u32;
        }
        iunlock((*f).ip);
        r
      }
      FileType::FD_NONE => panic("fileread"),
    }
  }
}

// Write to file f.
// addr is a user virtual address.
pub fn filewrite(f: *mut File, addr: u64, n: i32) -> i32 {
  unsafe {
    if !(*f).writable || n < 0 {
      return -1;
    }

    match (*f).typ {
      FileType::FD_PIPE => pipewrite((*f).pipe, addr, n),
      FileType::FD_DEVICE => {
        let major = (*f).major;
        if major < 0 || major as usize >= NDEV {
          return -1;
        }
        match DEVSW[major as usize].write {
          Some(write) => write(true, addr, n),
          None => -1,
        }
      }
      FileType::FD_INODE => {
        // write a few blocks at a time to avoid exceeding
        // the maximum log transaction size, including
        // i-node, indirect block, allocation blocks,
        // and 2 blocks of slop for non-aligned writes.
        let max = (((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE) as i32;
        let mut i = 0;
        while i < n {
          let mut n1 = n - i;
          if n1 > max {
            n1 = max;
          }

          begin_op();
          ilock((*f).ip);
          let r = writei((*f).ip, true, addr + i as u64, (*f).off, n1 as u32);
          if r > 0 {
            (*f).off += r as u32;
          }
          iunlock((*f).ip);
          end_op();

          if r != n1 {
            // error from writei
            break;
          }
          i += r;
        }
        if i == n {
          n
        } else {
          -1
        }
      }
      FileType::FD_NONE => panic("filewrite"),
    }
  }
}
