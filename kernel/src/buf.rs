use crate::fs::BSIZE;
use crate::sleeplock::Sleeplock;

pub struct Buf {
  valid: i32,
  disk: i32,
  dev: u32,
  blockno: u32,
  lock: Sleeplock,
  refcnt: u32,
  prev: Option<*mut Buf>,
  next: Option<*mut Buf>,
  data: [u8; BSIZE],
}