// Mutual exclusion spin locks.

use core::hint::spin_loop;
use core::ptr::null_mut;
use core::sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering};

use crate::printf::panic;
use crate::proc::{mycpu, Cpu};
use crate::riscv::{intr_get, intr_off, intr_on};

pub struct Spinlock {
  locked: AtomicBool,    // Is the lock held?

  // For debugging:
  name: &'static str,    // Name of lock.
  cpu: AtomicPtr<Cpu>,   // The cpu holding the lock.
}

impl Spinlock {
  pub const fn new() -> Self {
    Spinlock {
      locked: AtomicBool::new(false),
      name: "",
      cpu: AtomicPtr::new(null_mut()),
    }
  }
}

pub fn initlock(lk: *mut Spinlock, name: &'static str) {
  unsafe {
    (*lk).name = name;
    (*lk).locked.store(false, Ordering::Relaxed);
    (*lk).cpu.store(null_mut(), Ordering::Relaxed);
  }
}

// Acquire the lock.
// Loops (spins) until the lock is acquired.
pub fn acquire(lk: *mut Spinlock) {
  push_off(); // disable interrupts to avoid deadlock.
  if holding(lk) {
    panic("acquire");
  }

  unsafe {
    // On RISC-V, swap turns into an atomic swap:
    //   amoswap.w.aq a5, a5, (s1)
    while (*lk).locked.swap(true, Ordering::Acquire) {
      spin_loop();
    }

    // Tell the compiler and the processor to not move loads or stores
    // past this point, to ensure that the critical section's memory
    // references happen strictly after the lock is acquired.
    fence(Ordering::SeqCst);

    // Record info about lock acquisition for holding() and debugging.
    (*lk).cpu.store(mycpu(), Ordering::Relaxed);
  }
}

// Release the lock.
pub fn release(lk: *mut Spinlock) {
  if !holding(lk) {
    panic("release");
  }

  unsafe {
    (*lk).cpu.store(null_mut(), Ordering::Relaxed);

    // Tell the compiler and the CPU to not move loads or stores
    // past this point, to ensure that all the stores in the critical
    // section are visible to other CPUs before the lock is released,
    // and that loads in the critical section occur strictly before
    // the lock is released.
    fence(Ordering::SeqCst);

    // Release the lock.
    (*lk).locked.store(false, Ordering::Release);
  }

  pop_off();
}

// Check whether this cpu is holding the lock.
// Interrupts must be off.
pub fn holding(lk: *mut Spinlock) -> bool {
  unsafe {
    (*lk).locked.load(Ordering::Relaxed) && (*lk).cpu.load(Ordering::Relaxed) == mycpu()
  }
}

// push_off/pop_off are like intr_off()/intr_on() except that they are matched:
// it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
// are initially off, then push_off, pop_off leaves them off.

pub fn push_off() {
  let old = intr_get();

  // disable interrupts to prevent an involuntary context
  // switch while using mycpu().
  intr_off();

  let c = mycpu();
  unsafe {
    if (*c).noff == 0 {
      (*c).intena = old;
    }
    (*c).noff += 1;
  }
}

pub fn pop_off() {
  let c = mycpu();
  if intr_get() {
    panic("pop_off - interruptible");
  }
  unsafe {
    if (*c).noff < 1 {
      panic("pop_off");
    }
    (*c).noff -= 1;
    if (*c).noff == 0 && (*c).intena {
      intr_on();
    }
  }
}
