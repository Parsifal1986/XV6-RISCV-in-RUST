use crate::{param::LOGBLOCKS, spinlock::Spinlock};

struct Logheader {
  n: i32,
  block: [i32; LOGBLOCKS as usize],
}

struct Log {
  lock: Spinlock,
  start: i32,
  outstanding: i32,
  committing: i32,
  dev: i32,
  lh: Logheader,
}

pub fn begin_op() {
  todo!()
}

pub fn end_op() {
  todo!()
}