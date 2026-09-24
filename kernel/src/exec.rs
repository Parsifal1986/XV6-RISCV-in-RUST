use core::mem::MaybeUninit;
use core::ptr::null_mut;

use crate::elf::{Elfhdr, Proghdr, ELF_MAGIC, ELF_PROG_LOAD};
use crate::file::Inode;
use crate::fs::{ilock, iunlockput, namei, readi};
use crate::log::{begin_op, end_op};
use crate::param::{MAXARG, USERSTACK};
use crate::printf::panic;
use crate::proc::{myproc, proc_freepagetable, proc_pagetable};
use crate::riscv::{PagetableT, PGROUNDUP, PGSIZE, PTE_W, PTE_X};
use crate::string::{cstr, safestrcpy, strlen};
use crate::vm::{copyout, uvmalloc, uvmclear, walkaddr};

// map ELF permissions to PTE permission bits.
pub fn flags2perm(flags: u32) -> u64 {
  let mut perm = 0;
  if flags & 0x1 != 0 {
    perm = PTE_X;
  }
  if flags & 0x2 != 0 {
    perm |= PTE_W;
  }
  perm
}

// clean up after a failed exec.
fn bad(pagetable: PagetableT, sz: u64, ip: *mut Inode) -> i32 {
  if !pagetable.is_null() {
    proc_freepagetable(pagetable, sz);
  }
  if !ip.is_null() {
    iunlockput(ip);
    end_op();
  }
  -1
}

//
// the implementation of the exec() system call.
// argv is a null-terminated array of pointers to
// NUL-terminated argument strings.
//
pub fn kexec(path: &[u8], argv: &[*const u8]) -> i32 {
  let path = cstr(path);
  let mut sz: u64 = 0;
  let mut ustack = [0u64; MAXARG + 1];
  let mut pagetable: PagetableT = null_mut();
  let mut p = myproc();

  begin_op();

  // Open the executable file.
  let ip = namei(path);
  if ip.is_null() {
    end_op();
    return -1;
  }
  ilock(ip);

  // Read the ELF header.
  let mut elf = MaybeUninit::<Elfhdr>::zeroed();
  let elfsz = size_of::<Elfhdr>() as u32;
  if readi(ip, false, elf.as_mut_ptr() as u64, 0, elfsz) != elfsz as i32 {
    return bad(pagetable, sz, ip);
  }
  let elf = unsafe { elf.assume_init() };

  // Is this really an ELF file?
  if elf.magic != ELF_MAGIC {
    return bad(pagetable, sz, ip);
  }

  pagetable = proc_pagetable(p);
  if pagetable.is_null() {
    return bad(pagetable, sz, ip);
  }

  // Load program into memory.
  let phsz = size_of::<Proghdr>() as u32;
  let mut off = elf.phoff as u32;
  for _ in 0..elf.phnum {
    let mut ph = MaybeUninit::<Proghdr>::zeroed();
    if readi(ip, false, ph.as_mut_ptr() as u64, off, phsz) != phsz as i32 {
      return bad(pagetable, sz, ip);
    }
    let ph = unsafe { ph.assume_init() };
    off += phsz;
    if ph.typ != ELF_PROG_LOAD {
      continue;
    }
    if ph.memsz < ph.filesz {
      return bad(pagetable, sz, ip);
    }
    if ph.vaddr.wrapping_add(ph.memsz) < ph.vaddr {
      return bad(pagetable, sz, ip);
    }
    if ph.vaddr % PGSIZE != 0 {
      return bad(pagetable, sz, ip);
    }
    let sz1 = uvmalloc(pagetable, sz, ph.vaddr + ph.memsz, flags2perm(ph.flags));
    if sz1 == 0 {
      return bad(pagetable, sz, ip);
    }
    sz = sz1;
    if loadseg(pagetable, ph.vaddr, ip, ph.off as u32, ph.filesz as u32) < 0 {
      return bad(pagetable, sz, ip);
    }
  }
  iunlockput(ip);
  end_op();

  p = myproc();
  let oldsz = unsafe { (*p).sz };

  // Allocate some pages at the next page boundary.
  // Make the first inaccessible as a stack guard.
  // Use the rest as the user stack.
  sz = PGROUNDUP(sz);
  let sz1 = uvmalloc(pagetable, sz, sz + (USERSTACK + 1) * PGSIZE, PTE_W);
  if sz1 == 0 {
    return bad(pagetable, sz, null_mut());
  }
  sz = sz1;
  uvmclear(pagetable, sz - (USERSTACK + 1) * PGSIZE);
  let mut sp = sz;
  let stackbase = sp - USERSTACK * PGSIZE;

  // Copy argument strings into new stack, remember their
  // addresses in ustack[].
  let mut argc = 0;
  while !argv[argc].is_null() {
    if argc >= MAXARG {
      return bad(pagetable, sz, null_mut());
    }
    let len = strlen(argv[argc]) as u64 + 1;
    sp -= len;
    sp -= sp % 16; // riscv sp must be 16-byte aligned
    if sp < stackbase {
      return bad(pagetable, sz, null_mut());
    }
    if copyout(pagetable, sp, argv[argc], len) < 0 {
      return bad(pagetable, sz, null_mut());
    }
    ustack[argc] = sp;
    argc += 1;
  }
  ustack[argc] = 0;

  // push a copy of ustack[], the array of argv[] pointers.
  let ulen = (argc as u64 + 1) * size_of::<u64>() as u64;
  sp -= ulen;
  sp -= sp % 16;
  if sp < stackbase {
    return bad(pagetable, sz, null_mut());
  }
  if copyout(pagetable, sp, ustack.as_ptr() as *const u8, ulen) < 0 {
    return bad(pagetable, sz, null_mut());
  }

  unsafe {
    // a0 and a1 contain arguments to user main(argc, argv)
    // argc is returned via the system call return
    // value, which goes in a0.
    (*(*p).trapframe).a1 = sp;

    // Save program name for debugging.
    let last = match path.iter().rposition(|&c| c == b'/') {
      Some(pos) => &path[pos + 1..],
      None => path,
    };
    safestrcpy(&mut (*p).name, last);

    // Commit to the user image.
    let oldpagetable = (*p).pagetable;
    (*p).pagetable = pagetable;
    (*p).sz = sz;
    (*(*p).trapframe).epc = elf.entry; // initial program counter = _start
    (*(*p).trapframe).sp = sp; // initial stack pointer
    proc_freepagetable(oldpagetable, oldsz);
  }

  argc as i32 // this ends up in a0, the first argument to main(argc, argv)
}

// Load an ELF program segment into pagetable at virtual address va.
// va must be page-aligned
// and the pages from va to va+sz must already be mapped.
// Returns 0 on success, -1 on failure.
fn loadseg(pagetable: PagetableT, va: u64, ip: *mut Inode, offset: u32, sz: u32) -> i32 {
  let mut i: u32 = 0;
  while i < sz {
    let pa = walkaddr(pagetable, va + i as u64);
    if pa == 0 {
      panic("loadseg: address should exist");
    }
    let n = if sz - i < PGSIZE as u32 { sz - i } else { PGSIZE as u32 };
    if readi(ip, false, pa, offset + i, n) != n as i32 {
      return -1;
    }
    i += PGSIZE as u32;
  }

  0
}
