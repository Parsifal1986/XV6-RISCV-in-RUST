use crate::spinlock::{Spinlock, initlock};
use crate::memlayout::PHYSTOP;

extern "C" {
  static etext: u8;
  static end: u8;
}

pub fn kinit() {
  initlock(&mut KMEM.lock, Some("kmem"));
  freerange(unsafe{&end as *const u8 as u64}, PHYSTOP);
}

pub fn freerange(pa_start: u64, pa_end: u64) {
  todo!();
}

pub struct Run {
  pub next: Option<&'static mut Run>,
}

pub struct KMem {
  pub lock: Spinlock,
  pub freelist: Option<&'static mut Run>,
}

pub fn kalloc() -> u64 {
  todo!();
}