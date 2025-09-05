use crate::spinlock::Spinlock;

pub struct Sleeplock {
  locked: u32,
  lk: Spinlock,

  name: &'static str,
  pid: i32,
}

impl Sleeplock {
  pub const fn new() -> Self {
    Sleeplock {
      locked: 0,
      lk: Spinlock::new(),
      name: "",
      pid: -1,
    }
  }
}