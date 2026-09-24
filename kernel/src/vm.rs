use core::ptr::{copy, null_mut, write_bytes};

use crate::kalloc::{kalloc, kfree};
use crate::memlayout::{KERNBASE, PHYSTOP, PLIC, TRAMPOLINE, UART0, VIRTIO0};
use crate::printf::panic;
use crate::proc::{myproc, proc_mapstacks};
use crate::riscv::{
  sfence_vma, w_satp, PagetableT, PteT, MAKE_SATP, MAXVA, PA2PTE, PGROUNDDOWN, PGROUNDUP, PGSIZE, PTE2PA, PTE_FLAGS, PTE_R,
  PTE_U, PTE_V, PTE_W, PTE_X, PX,
};

extern "C" {
  // kernel.ld sets this to end of kernel code.
  static etext: u8;

  // trampoline.S
  static trampoline: [u8; 0];
}

pub const SBRK_EAGER: i32 = 1;
pub const SBRK_LAZY: i32 = 2;

/*
 * the kernel's page table.
 */
static mut KERNEL_PAGETABLE: PagetableT = null_mut();

// Make a direct-map page table for the kernel.
pub fn kvmmake() -> PagetableT {
  let kpgtbl = kalloc() as PagetableT;
  if kpgtbl.is_null() {
    panic("kvmmake");
  }
  unsafe {
    write_bytes(kpgtbl as *mut u8, 0, PGSIZE as usize);
  }

  // uart registers
  kvmmap(kpgtbl, UART0, UART0, PGSIZE, PTE_R | PTE_W);

  // virtio mmio disk interface
  kvmmap(kpgtbl, VIRTIO0, VIRTIO0, PGSIZE, PTE_R | PTE_W);

  // PLIC
  kvmmap(kpgtbl, PLIC, PLIC, 0x4000000, PTE_R | PTE_W);

  let etextaddr = &raw const etext as u64;

  // map kernel text executable and read-only.
  kvmmap(kpgtbl, KERNBASE, KERNBASE, etextaddr - KERNBASE, PTE_R | PTE_X);

  // map kernel data and the physical RAM we'll make use of.
  kvmmap(kpgtbl, etextaddr, etextaddr, PHYSTOP - etextaddr, PTE_R | PTE_W);

  // map the trampoline for trap entry/exit to
  // the highest virtual address in the kernel.
  let tramp = unsafe { trampoline.as_ptr() as u64 };
  kvmmap(kpgtbl, TRAMPOLINE, tramp, PGSIZE, PTE_R | PTE_X);

  // allocate and map a kernel stack for each process.
  proc_mapstacks(kpgtbl);

  kpgtbl
}

// add a mapping to the kernel page table.
// only used when booting.
// does not flush TLB or enable paging.
pub fn kvmmap(kpgtbl: PagetableT, va: u64, pa: u64, sz: u64, perm: u64) {
  if mappages(kpgtbl, va, sz, pa, perm) != 0 {
    panic("kvmmap");
  }
}

// Initialize the KERNEL_PAGETABLE, shared by all CPUs.
pub fn kvminit() {
  unsafe {
    KERNEL_PAGETABLE = kvmmake();
  }
}

// Switch the current CPU's h/w page table register to
// the kernel's page table, and enable paging.
pub fn kvminithart() {
  // wait for any previous writes to the page table memory to finish.
  sfence_vma();

  w_satp(MAKE_SATP(unsafe { KERNEL_PAGETABLE as u64 }));

  // flush stale entries from the TLB.
  sfence_vma();
}

// Return the address of the PTE in page table pagetable
// that corresponds to virtual address va.  If alloc is true,
// create any required page-table pages.
//
// The risc-v Sv39 scheme has three levels of page-table
// pages. A page-table page contains 512 64-bit PTEs.
// A 64-bit virtual address is split into five fields:
//   39..63 -- must be zero.
//   30..38 -- 9 bits of level-2 index.
//   21..29 -- 9 bits of level-1 index.
//   12..20 -- 9 bits of level-0 index.
//    0..11 -- 12 bits of byte offset within the page.
pub fn walk(mut pagetable: PagetableT, va: u64, alloc: bool) -> *mut PteT {
  if va >= MAXVA {
    panic("walk");
  }

  unsafe {
    for level in (1..=2).rev() {
      let pte = pagetable.add(PX(level, va) as usize);
      if *pte & PTE_V != 0 {
        pagetable = PTE2PA(*pte) as PagetableT;
      } else {
        if !alloc {
          return null_mut();
        }
        pagetable = kalloc() as PagetableT;
        if pagetable.is_null() {
          return null_mut();
        }
        write_bytes(pagetable as *mut u8, 0, PGSIZE as usize);
        *pte = PA2PTE(pagetable as u64) | PTE_V;
      }
    }
    pagetable.add(PX(0, va) as usize)
  }
}

// Look up a virtual address, return the physical address,
// or 0 if not mapped.
// Can only be used to look up user pages.
pub fn walkaddr(pagetable: PagetableT, va: u64) -> u64 {
  if va >= MAXVA {
    return 0;
  }

  let pte = walk(pagetable, va, false);
  if pte.is_null() {
    return 0;
  }
  let pte = unsafe { *pte };
  if pte & PTE_V == 0 {
    return 0;
  }
  if pte & PTE_U == 0 {
    return 0;
  }
  PTE2PA(pte)
}

// Create PTEs for virtual addresses starting at va that refer to
// physical addresses starting at pa.
// va and size MUST be page-aligned.
// Returns 0 on success, -1 if walk() couldn't
// allocate a needed page-table page.
pub fn mappages(pagetable: PagetableT, va: u64, size: u64, mut pa: u64, perm: u64) -> i32 {
  if va % PGSIZE != 0 {
    panic("mappages: va not aligned");
  }

  if size % PGSIZE != 0 {
    panic("mappages: size not aligned");
  }

  if size == 0 {
    panic("mappages: size");
  }

  let mut a = va;
  let last = va + size - PGSIZE;
  loop {
    let pte = walk(pagetable, a, true);
    if pte.is_null() {
      return -1;
    }
    unsafe {
      if *pte & PTE_V != 0 {
        panic("mappages: remap");
      }
      *pte = PA2PTE(pa) | perm | PTE_V;
    }
    if a == last {
      break;
    }
    a += PGSIZE;
    pa += PGSIZE;
  }
  0
}

// create an empty user page table.
// returns null if out of memory.
pub fn uvmcreate() -> PagetableT {
  let pagetable = kalloc() as PagetableT;
  if pagetable.is_null() {
    return null_mut();
  }
  unsafe {
    write_bytes(pagetable as *mut u8, 0, PGSIZE as usize);
  }
  pagetable
}

// Remove npages of mappings starting from va. va must be
// page-aligned. It's OK if the mappings don't exist.
// Optionally free the physical memory.
pub fn uvmunmap(pagetable: PagetableT, va: u64, npages: u64, do_free: bool) {
  if va % PGSIZE != 0 {
    panic("uvmunmap: not aligned");
  }

  let mut a = va;
  while a < va + npages * PGSIZE {
    let pte = walk(pagetable, a, false);
    // leaf page table entry allocated? physical page allocated?
    if !pte.is_null() && unsafe { *pte } & PTE_V != 0 {
      if do_free {
        let pa = PTE2PA(unsafe { *pte });
        kfree(pa as *mut u8);
      }
      unsafe { *pte = 0 };
    }
    a += PGSIZE;
  }
}

// Allocate PTEs and physical memory to grow a process from oldsz to
// newsz, which need not be page aligned.  Returns new size or 0 on error.
pub fn uvmalloc(pagetable: PagetableT, oldsz: u64, newsz: u64, xperm: u64) -> u64 {
  if newsz < oldsz {
    return oldsz;
  }

  let oldsz = PGROUNDUP(oldsz);
  let mut a = oldsz;
  while a < newsz {
    let mem = kalloc();
    if mem.is_null() {
      uvmdealloc(pagetable, a, oldsz);
      return 0;
    }
    unsafe { write_bytes(mem, 0, PGSIZE as usize) };
    if mappages(pagetable, a, PGSIZE, mem as u64, PTE_R | PTE_U | xperm) != 0 {
      kfree(mem);
      uvmdealloc(pagetable, a, oldsz);
      return 0;
    }
    a += PGSIZE;
  }
  newsz
}

// Deallocate user pages to bring the process size from oldsz to
// newsz.  oldsz and newsz need not be page-aligned, nor does newsz
// need to be less than oldsz.  oldsz can be larger than the actual
// process size.  Returns the new process size.
pub fn uvmdealloc(pagetable: PagetableT, oldsz: u64, newsz: u64) -> u64 {
  if newsz >= oldsz {
    return oldsz;
  }

  if PGROUNDUP(newsz) < PGROUNDUP(oldsz) {
    let npages = (PGROUNDUP(oldsz) - PGROUNDUP(newsz)) / PGSIZE;
    uvmunmap(pagetable, PGROUNDUP(newsz), npages, true);
  }

  newsz
}

// Recursively free page-table pages.
// All leaf mappings must already have been removed.
fn freewalk(pagetable: PagetableT) {
  // there are 2^9 = 512 PTEs in a page table.
  for i in 0..512 {
    unsafe {
      let pte = *pagetable.add(i);
      if (pte & PTE_V) != 0 && (pte & (PTE_R | PTE_W | PTE_X)) == 0 {
        // this PTE points to a lower-level page table.
        let child = PTE2PA(pte);
        freewalk(child as PagetableT);
        *pagetable.add(i) = 0;
      } else if pte & PTE_V != 0 {
        panic("freewalk: leaf");
      }
    }
  }
  kfree(pagetable as *mut u8);
}

// Free user memory pages,
// then free page-table pages.
pub fn uvmfree(pagetable: PagetableT, sz: u64) {
  if sz > 0 {
    uvmunmap(pagetable, 0, PGROUNDUP(sz) / PGSIZE, true);
  }
  freewalk(pagetable);
}

// Given a parent process's page table, copy
// its memory into a child's page table.
// Copies both the page table and the
// physical memory.
// returns 0 on success, -1 on failure.
// frees any allocated pages on failure.
pub fn uvmcopy(old: PagetableT, new: PagetableT, sz: u64) -> i32 {
  let mut i = 0;
  while i < sz {
    let pte = walk(old, i, false);
    // page table entry or physical page not allocated (lazy sbrk)?
    if !pte.is_null() && unsafe { *pte } & PTE_V != 0 {
      let pa = PTE2PA(unsafe { *pte });
      let flags = PTE_FLAGS(unsafe { *pte });
      let mem = kalloc();
      if mem.is_null() {
        uvmunmap(new, 0, i / PGSIZE, true);
        return -1;
      }
      unsafe { copy(pa as *const u8, mem, PGSIZE as usize) };
      if mappages(new, i, PGSIZE, mem as u64, flags) != 0 {
        kfree(mem);
        uvmunmap(new, 0, i / PGSIZE, true);
        return -1;
      }
    }
    i += PGSIZE;
  }
  0
}

// mark a PTE invalid for user access.
// used by exec for the user stack guard page.
pub fn uvmclear(pagetable: PagetableT, va: u64) {
  let pte = walk(pagetable, va, false);
  if pte.is_null() {
    panic("uvmclear");
  }
  unsafe { *pte &= !PTE_U };
}

// Copy from kernel to user.
// Copy len bytes from src to virtual address dstva in a given page table.
// Return 0 on success, -1 on error.
pub fn copyout(pagetable: PagetableT, mut dstva: u64, mut src: *const u8, mut len: u64) -> i32 {
  while len > 0 {
    let va0 = PGROUNDDOWN(dstva);
    if va0 >= MAXVA {
      return -1;
    }

    let mut pa0 = walkaddr(pagetable, va0);
    if pa0 == 0 {
      pa0 = vmfault(pagetable, va0, false);
      if pa0 == 0 {
        return -1;
      }
    }

    // forbid copyout over read-only user text pages.
    let pte = walk(pagetable, va0, false);
    if unsafe { *pte } & PTE_W == 0 {
      return -1;
    }

    let mut n = PGSIZE - (dstva - va0);
    if n > len {
      n = len;
    }
    unsafe {
      copy(src, (pa0 + (dstva - va0)) as *mut u8, n as usize);
      src = src.add(n as usize);
    }

    len -= n;
    dstva = va0 + PGSIZE;
  }
  0
}

// Copy from user to kernel.
// Copy len bytes to dst from virtual address srcva in a given page table.
// Return 0 on success, -1 on error.
pub fn copyin(pagetable: PagetableT, mut dst: *mut u8, mut srcva: u64, mut len: u64) -> i32 {
  while len > 0 {
    let va0 = PGROUNDDOWN(srcva);
    let mut pa0 = walkaddr(pagetable, va0);
    if pa0 == 0 {
      pa0 = vmfault(pagetable, va0, false);
      if pa0 == 0 {
        return -1;
      }
    }
    let mut n = PGSIZE - (srcva - va0);
    if n > len {
      n = len;
    }
    unsafe {
      copy((pa0 + (srcva - va0)) as *const u8, dst, n as usize);
      dst = dst.add(n as usize);
    }

    len -= n;
    srcva = va0 + PGSIZE;
  }
  0
}

// Copy a null-terminated string from user to kernel.
// Copy bytes to dst from virtual address srcva in a given page table,
// until a '\0', or max.
// Return 0 on success, -1 on error.
pub fn copyinstr(pagetable: PagetableT, mut dst: *mut u8, mut srcva: u64, mut max: u64) -> i32 {
  let mut got_null = false;

  while !got_null && max > 0 {
    let va0 = PGROUNDDOWN(srcva);
    let mut pa0 = walkaddr(pagetable, va0);
    if pa0 == 0 {
      pa0 = vmfault(pagetable, va0, false);
      if pa0 == 0 {
        return -1;
      }
    }
    let mut n = PGSIZE - (srcva - va0);
    if n > max {
      n = max;
    }

    let mut p = (pa0 + (srcva - va0)) as *const u8;
    unsafe {
      while n > 0 {
        if *p == 0 {
          *dst = 0;
          got_null = true;
          break;
        } else {
          *dst = *p;
        }
        n -= 1;
        max -= 1;
        p = p.add(1);
        dst = dst.add(1);
      }
    }

    srcva = va0 + PGSIZE;
  }
  if got_null {
    0
  } else {
    -1
  }
}

// allocate and map user memory if process is referencing a page
// that was lazily allocated in sys_sbrk().
// returns 0 if va is invalid or already mapped, or if
// out of physical memory, and physical address if successful.
pub fn vmfault(pagetable: PagetableT, va: u64, _read: bool) -> u64 {
  let p = myproc();

  if va >= unsafe { (*p).sz } {
    return 0;
  }
  let va = PGROUNDDOWN(va);
  if ismapped(pagetable, va) {
    return 0;
  }
  let mem = kalloc();
  if mem.is_null() {
    return 0;
  }
  unsafe { write_bytes(mem, 0, PGSIZE as usize) };
  if mappages(pagetable, va, PGSIZE, mem as u64, PTE_W | PTE_U | PTE_R) != 0 {
    kfree(mem);
    return 0;
  }
  mem as u64
}

pub fn ismapped(pagetable: PagetableT, va: u64) -> bool {
  let pte = walk(pagetable, va, false);
  if pte.is_null() {
    return false;
  }
  unsafe { *pte & PTE_V != 0 }
}
