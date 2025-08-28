use std::u64;

use crate::riscv::{intr_get, intr_off, intr_on};
use crate::proc::mycpu;

pub fn initlock(lk: &mut Spinlock, name: Option<&'static str>) {
  lk.name = name;
  lk.locked = false;
  lk.cpu = 0;
}

pub fn acquire(lk: &mut Spinlock) {
  

  while lk.locked {
    // spin
  }
  lk.locked = true;
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
    todo!();
  }
  if c.noff < 1 {
    todo!();
  }
  if c.noff == 0 && c.intena != 0 {
    intr_on();
  }
}

pub struct Spinlock {
  locked: bool,
  name: Option<&'static str>,
  cpu: u64
}