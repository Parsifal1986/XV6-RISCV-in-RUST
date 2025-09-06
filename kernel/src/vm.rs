use crate::memlayout::{KERNBASE, PHYSTOP, PLIC, TRAMPOLINE, UART0, VIRTIO0};
use crate::proc::proc_mapstacks;
use crate::riscv::{sfence_vma, w_satp, PagetableT, PteT, MAKE_SATP, MAXVA, PA2PTE, PGSIZE, PTE2PA, PTE_R, PTE_U, PTE_V, PTE_W, PTE_X, PX};
use crate::kalloc::kalloc;
use crate::defs::panic;
use core::ptr::write_bytes;
use core::ptr::null_mut;

extern "C" {
  static etext: u8;
}

static mut KERNEL_PAGETABLE: PagetableT = 0 as PagetableT;

pub fn kvmmake() -> PagetableT {
  let kpgtbl = kalloc() as PagetableT;

  unsafe {
    write_bytes(kpgtbl as *mut u8, 0, PGSIZE as usize);
  }

  kvmmap(kpgtbl, UART0, UART0, PGSIZE, (PTE_R | PTE_W) as i32);

  kvmmap(kpgtbl, VIRTIO0, VIRTIO0, PGSIZE, (PTE_R | PTE_W) as i32);

  kvmmap(kpgtbl, PLIC, PLIC, 0x400000, (PTE_R | PTE_W) as i32);

  let etextaddr = unsafe {
      etext as *const u8 as u64
  };

  kvmmap(kpgtbl, KERNBASE, KERNBASE, etextaddr - KERNBASE, (PTE_R | PTE_X) as i32);

  kvmmap(kpgtbl, etextaddr, etextaddr, PHYSTOP - etextaddr, PTE_R as i32);

  kvmmap(kpgtbl, TRAMPOLINE, TRAMPOLINE, PGSIZE, (PTE_R | PTE_X) as i32);

  proc_mapstacks(kpgtbl);

  kpgtbl
}

pub fn kvmmap(kpgtbl: PagetableT, va: u64, pa: u64, sz: u64, perm: i32) {
  if mappage(kpgtbl, va, sz, pa, perm) != 0 {
    panic("kvmmap".as_bytes());
  }
}

pub fn walk(mut pagetable: PagetableT, va: u64, alloc: i32) -> Option<&'static mut PteT> {
  if va >= MAXVA {
    panic("walk".as_bytes());
  }

  for i in 2..0 {
    let pte: &mut PteT = unsafe { &mut *pagetable.add(PX(i, va) as usize)};

    if *pte != 0 & PTE_V as u64 {
      return Some(pte);
    } else {
      if alloc == 0 || {pagetable = kalloc() as PagetableT; pagetable} as u64 == 0 {
        return None;
      }
      unsafe {
        write_bytes(pagetable, 0, PGSIZE as usize);
      }
      *pte = PA2PTE(pagetable as u64) | PTE_V as u64;
    }
  }
  Some(unsafe {
    &mut *pagetable.add(PX(0, va) as usize)
  })
}

pub fn walkaddr(pagetable: PagetableT, va: u64) -> u64 {
  if va >= MAXVA {
    return 0;
  }

  let pte = walk(pagetable, va, 0);
  if pte.is_none() {
    return 0;
  }

  let pte_value = *pte.unwrap();
  if pte_value & PTE_V as u64 == 0 {
    return 0;
  }
  if pte_value & PTE_U as u64 == 0 {
    return 0;
  }
  PTE2PA(pte_value)
}

pub fn mappage(pagetable: PagetableT, va: u64, size: u64, mut pa: u64, perm: i32) -> i32 {
  let (mut a, last): (u64, u64);
  let mut pte: Option<&'static mut PteT>;

  if va % PGSIZE != 0 {
    panic("mappage: va not aligned".as_bytes());
  }
  
  if size % PGSIZE != 0 {
    panic("mappage: size not aligned".as_bytes());
  }

  if size == 0 {
    panic("mappage: size zero".as_bytes());
  }
  a = va;
  last = va + size - PGSIZE;
  loop {
    pte = walk(pagetable, a, 1);
    let pte_value : &mut u64 = match pte {
      Some(p) => p,
      None => return -1,
    };
    if *pte_value & PTE_V as u64 != 0 {
      panic("mappage: remap".as_bytes());
    }
    *pte_value = PA2PTE(pa) | perm as u64 | PTE_V;
    if a == last {
      break;
    }
    a += PGSIZE;
    pa += PGSIZE;
  }
  0
}

pub fn kvminit() {
  unsafe { 
    KERNEL_PAGETABLE = kvmmake()
  };
}

pub fn kvminithart() {
  sfence_vma();

  w_satp(MAKE_SATP(unsafe { KERNEL_PAGETABLE as u64 }));

  sfence_vma();
}

pub fn uvmcreate() -> PagetableT {
  let pagetable = kalloc() as PagetableT;
  if pagetable as u64 == 0 {
    return null_mut();
  }
  unsafe {
    write_bytes(pagetable as *mut u8, 0, PGSIZE as usize);
  }
  pagetable
}

pub fn uvmalloc(pagetable: PagetableT, oldsz: u64, newsz: u64, xperm: i32) -> u64 {
  todo!()
}

pub fn uvmclear(pagetable: PagetableT, va: u64) {
  todo!()
}

pub fn copyout(pagetable: PagetableT, va: u64, src: u64, len: u64) -> i32 {
  todo!()
}