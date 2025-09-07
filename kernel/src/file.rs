use crate::fs::NDIRECT;
use crate::sleeplock::Sleeplock;
use crate::pipe::Pipe;

#[repr(C)]
pub struct File {
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
pub struct Devsw {
  pub(crate) read: fn(i32, u64, i32) -> i32,
  pub(crate) write: fn(i32, u64, i32) -> i32,
}

impl Devsw{
  pub const fn new() -> Self {
    Devsw { read: Self::dummy_read, write: Self::dummy_write }
  }

  const fn dummy_read(_: i32, _: u64, _: i32) -> i32 {
    -1
  }

  const fn dummy_write(_: i32, _: u64, _: i32) -> i32 {
    -1
  }
}

pub static mut DEVSW: [Devsw; 10] = [const { Devsw::new() }; 10];

pub const CONSOLE: i32 = 1;

impl File {
  pub const fn new() -> Self {
    File {
      file_type: FileType::None,
      ref_count: 0,
      readable: 0,
      writable: 0,
      pipe: None,
      ip: None,
      off: 0,
      major: 0,
    }
  }
}