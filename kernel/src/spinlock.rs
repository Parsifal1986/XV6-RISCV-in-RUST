use core::sync::atomic::{AtomicBool, Ordering};
use std::hint::spin_loop;
use std::sync::atomic::fence;

use crate::riscv::{intr_get, intr_off, intr_on};
use crate::proc::{mycpu, Cpu};
use crate::defs::panic;

pub struct Spinlock {
  locked: AtomicBool,
  name: Option<&'static str>,
  cpu: Option<&'static Cpu>,
}

impl Spinlock {
  pub const fn new() -> Self {
    Spinlock {
      locked: AtomicBool::new(false),
      name: None,
      cpu: None,
    }
  }
}

pub fn initlock(lk: &mut Spinlock, name: Option<&'static str>) {
  lk.name = name;
  lk.locked = AtomicBool::new(false);
  lk.cpu = None;
}

pub fn acquire(lk: &mut Spinlock) {
  push_off();
  if holding(lk) {
    panic("acquire");
  }

  while lk.locked.swap(true, Ordering::Acquire) {
    spin_loop();
  }
  
  fence(Ordering::SeqCst);

  lk.cpu = Some(mycpu());
}

pub fn release(lk: &mut Spinlock) {
  if !holding(lk) {
    panic("release");
  }

  lk.cpu = None;

  fence(Ordering::SeqCst);
  lk.locked.store(false, Ordering::Release);

  pop_off();
}

fn holding(lk: &Spinlock) -> bool {
  lk.locked.load(Ordering::Acquire) && lk.cpu.map(|c| std::ptr::eq(c, mycpu())).unwrap_or(false)
}

fn push_off() {
  let old: u64 = intr_get();

  intr_off();

  if mycpu().noff == 0 {
    mycpu().intena = old;
  }
  mycpu().noff += 1;
}

fn pop_off() {
  let c = mycpu();
  if intr_get() != 0 {
    panic("pop_off - interruptible");
  }
  if c.noff < 1 {
    panic("pop_off");
  }
  c.noff -= 1;
  if c.noff == 0 && c.intena != 0 {
    intr_on();
  }
}