use crate::fs::NDIRECT;
use crate::sleeplock::Sleeplock;
use crate::pipe::Pipe;

#[repr(C)]
struct File {
  file_type: FileType,
  ref_count: i32,
  readable: u8,
  writable: u8,
  pipe: Option<*mut Pipe>,
  ip: Option<*mut Inode>,
  off: u32,
  major: i16,
}

#[repr(C)]
enum FileType {
  None,
  Pipe,
  Inode,
  Device,
}

#[repr(C)]
pub(crate) struct Inode {
  dev: u32,
  inum: u32,
  ref_count: i32,
  lock: Sleeplock,
  valid: i32,
  inode_type: i16,
  major: i16,
  minor: i16,
  nlink: i16,
  size: u32,
  addrs: [u32; NDIRECT + 1],
}

impl Inode {
  pub const fn new() -> Self {
    Inode {
      dev: 0,
      inum: 0,
      ref_count: 0,
      lock: Sleeplock::new(),
      valid: 0,
      inode_type: 0,
      major: 0,
      minor: 0,
      nlink: 0,
      size: 0,
      addrs: [0; NDIRECT + 1],
    }
  }
}

#[repr(C)]
struct Devsw {
  read: fn(i32, u64, i32) -> i32,
  write: fn(i32, u64, i32) -> i32,
}

extern "C" {
  static mut devsw: [Devsw; 256];
}

const CONSOLE: i32 = 1;
