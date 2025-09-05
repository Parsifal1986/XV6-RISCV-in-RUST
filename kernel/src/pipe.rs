use crate::spinlock::Spinlock;

pub const PIPESIZE: usize = 512;

pub struct Pipe {
  lock: Spinlock,
  data: [u8; PIPESIZE],
  nread: u32,
  nwrite: u32,
  readopen: i32,
  writeopen: i32,
}