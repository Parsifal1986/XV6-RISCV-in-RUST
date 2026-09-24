// Sleeping locks

use crate::proc::{myproc, sleep, wakeup};
use crate::spinlock::{acquire, initlock, release, Spinlock};

// Long-term locks for processes
pub struct Sleeplock {
  locked: bool,        // Is the lock held?
  lk: Spinlock,        // spinlock protecting this sleep lock

  // For debugging:
  name: &'static str,  // Name of lock.
  pid: i32,            // Process holding lock
}

impl Sleeplock {
  pub const fn new() -> Self {
    Sleeplock {
      locked: false,
      lk: Spinlock::new(),
      name: "",
      pid: 0,
    }
  }
}

pub fn initsleeplock(lk: *mut Sleeplock, name: &'static str) {
  unsafe {
    initlock(&raw mut (*lk).lk, "sleep lock");
    (*lk).name = name;
    (*lk).locked = false;
    (*lk).pid = 0;
  }
}

pub fn acquiresleep(lk: *mut Sleeplock) {
  unsafe {
    acquire(&raw mut (*lk).lk);
    while core::ptr::read_volatile(&raw const (*lk).locked) {
      sleep(lk as *const u8, &raw mut (*lk).lk);
    }
    (*lk).locked = true;
    (*lk).pid = (*myproc()).pid;
    release(&raw mut (*lk).lk);
  }
}

pub fn releasesleep(lk: *mut Sleeplock) {
  unsafe {
    acquire(&raw mut (*lk).lk);
    (*lk).locked = false;
    (*lk).pid = 0;
    wakeup(lk as *const u8);
    release(&raw mut (*lk).lk);
  }
}

pub fn holdingsleep(lk: *mut Sleeplock) -> bool {
  unsafe {
    acquire(&raw mut (*lk).lk);
    let r = (*lk).locked && (*lk).pid == (*myproc()).pid;
    release(&raw mut (*lk).lk);
    r
  }
}
