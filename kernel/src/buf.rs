use core::ptr::null_mut;

use crate::fs::BSIZE;
use crate::sleeplock::Sleeplock;

#[repr(C)]
pub struct Buf {
  pub valid: bool,     // has data been read from disk?
  pub disk: bool,      // does disk "own" buf?
  pub dev: u32,
  pub blockno: u32,
  pub lock: Sleeplock,
  pub refcnt: u32,
  pub prev: *mut Buf,  // LRU cache list
  pub next: *mut Buf,
  pub data: [u8; BSIZE],
}

impl Buf {
  pub const fn new() -> Self {
    Buf {
      valid: false,
      disk: false,
      dev: 0,
      blockno: 0,
      lock: Sleeplock::new(),
      refcnt: 0,
      prev: null_mut(),
      next: null_mut(),
      data: [0; BSIZE],
    }
  }
}
