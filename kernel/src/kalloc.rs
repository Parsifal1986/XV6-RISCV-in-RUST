use crate::riscv::{PGROUNDUP, PGSIZE};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::memlayout::PHYSTOP;
use core::ptr::null_mut;
use std::ptr::write_bytes;

extern "C" {
  static etext: u8;
  static end: u8;
}

pub struct Run {
  pub next: *mut Run,
}

pub struct KMem {
  pub lock: Spinlock,
  pub freelist: *mut Run,
} // 改进方向：受保护的数据放到Spinlock中，可以避免unsafe

impl KMem {
  pub const fn new() -> Self {
    KMem {
      lock: Spinlock::new(),
      freelist: null_mut(),
    }
  }
}

static mut KMEM: KMem = KMem::new();

pub fn kinit() {
  initlock(unsafe { &mut KMEM.lock }, Some("kmem"));
  freerange(unsafe{&end as *const u8}, PHYSTOP as *const u8);
}

pub fn freerange(pa_start: *const u8, pa_end: *const u8) {
  let mut p = PGROUNDUP(pa_start as u64);
  while p + PGSIZE <= pa_end as u64 {
    kfree(p as *mut u8);
    p += PGSIZE;
  }
}

pub fn kfree(pa: *mut u8) {
  let r: *mut Run;

  if pa as u64 % PGSIZE != 0 || pa < unsafe { &end as *const u8 } as *mut u8 || pa as u64 >= PHYSTOP {
    panic!("kfree");
  }

  unsafe { write_bytes(pa, 1, PGSIZE as usize) };

  r = pa as *mut Run;

  acquire(unsafe { &mut KMEM.lock });
  unsafe {
    (*r).next = KMEM.freelist;
    KMEM.freelist = r;
  }

  release(unsafe { &mut KMEM.lock });
}

pub fn kalloc() -> u64 {
  todo!();
}