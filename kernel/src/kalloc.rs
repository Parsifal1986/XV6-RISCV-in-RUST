// Physical memory allocator, for user processes,
// kernel stacks, page-table pages,
// and pipe buffers. Allocates whole 4096-byte pages.

use core::ptr::{null_mut, write_bytes};

use crate::memlayout::PHYSTOP;
use crate::printf::panic;
use crate::riscv::{PGROUNDUP, PGSIZE};
use crate::spinlock::{acquire, initlock, release, Spinlock};

extern "C" {
  // first address after kernel.
  // defined by kernel.ld.
  static end: u8;
}

struct Run {
  next: *mut Run,
}

struct KMem {
  lock: Spinlock,
  freelist: *mut Run,
}

static mut KMEM: KMem = KMem {
  lock: Spinlock::new(),
  freelist: null_mut(),
};

fn end_addr() -> u64 {
  &raw const end as u64
}

pub fn kinit() {
  unsafe {
    initlock(&raw mut KMEM.lock, "kmem");
  }
  freerange(end_addr(), PHYSTOP);
}

fn freerange(pa_start: u64, pa_end: u64) {
  let mut p = PGROUNDUP(pa_start);
  while p + PGSIZE <= pa_end {
    kfree(p as *mut u8);
    p += PGSIZE;
  }
}

// Free the page of physical memory pointed at by pa,
// which normally should have been returned by a
// call to kalloc().  (The exception is when
// initializing the allocator; see kinit above.)
pub fn kfree(pa: *mut u8) {
  if (pa as u64) % PGSIZE != 0 || (pa as u64) < end_addr() || (pa as u64) >= PHYSTOP {
    panic("kfree");
  }

  unsafe {
    // Fill with junk to catch dangling refs.
    write_bytes(pa, 1, PGSIZE as usize);

    let r = pa as *mut Run;

    acquire(&raw mut KMEM.lock);
    (*r).next = KMEM.freelist;
    KMEM.freelist = r;
    release(&raw mut KMEM.lock);
  }
}

// Allocate one 4096-byte page of physical memory.
// Returns a pointer that the kernel can use.
// Returns null if the memory cannot be allocated.
pub fn kalloc() -> *mut u8 {
  unsafe {
    acquire(&raw mut KMEM.lock);
    let r = KMEM.freelist;
    if !r.is_null() {
      KMEM.freelist = (*r).next;
    }
    release(&raw mut KMEM.lock);

    if !r.is_null() {
      write_bytes(r as *mut u8, 5, PGSIZE as usize); // fill with junk
    }
    r as *mut u8
  }
}
