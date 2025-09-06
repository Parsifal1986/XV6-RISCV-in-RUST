use crate::defs::panic;
use crate::elf::{check_magic, Elfhdr, Proghdr, ELF_PROG_LOAD};
use crate::fs::{ilock, iunlockput, namei, readi};
use crate::log::{begin_op, end_op};
use crate::param::{MAXARG, USERSTACK};
use crate::proc::{myproc, proc_freepagetable, proc_pagetable};
use crate::riscv::{PagetableT, PGROUNDUP, PGSIZE, PTE_W, PTE_X};
use crate::file::Inode;
use crate::vm::{copyout, uvmalloc, uvmclear, walkaddr};
use core::mem::{size_of, zeroed};

pub fn flags2perm(flags: i32) -> u64 {
  let mut perm: u64 = 0;
  
  if flags & 0x1 != 0 {
    perm |= PTE_X;
  }
  if flags & 0x2 != 0 {
    perm |= PTE_W;
  }

  perm
}

#[inline(always)]
fn bad(pagetable: Option<PagetableT>, sz: u64, ip: Option<*mut Inode>) {
  if pagetable.is_some() {
    proc_freepagetable(pagetable.unwrap(), sz);
  }
  if ip.is_some() {
    iunlockput(ip.unwrap());
    end_op();
  }
}

pub fn kexec(path: &[u8], argv: &[*const u8]) -> i32 {
  let s: &[u8] = path;
  let last: &[u8];
  let (i, mut off): (i32, usize) = (0, 0);
  let (mut argc, mut sz) : (u64, u64) = (0, 0);
  let (mut sp, stackbase): (u64, u64);
  let mut ustack: [u64; MAXARG as usize] = [0; MAXARG as usize];
  let mut elf: Elfhdr = unsafe { zeroed() };
  let ip: *mut Inode;
  let mut ph: Proghdr = unsafe { zeroed() };
  let mut pagetable: Option<PagetableT> = None;
  let oldpagetable: Option<PagetableT>;
  let mut p = myproc();

  begin_op();

  ip = match namei(path) {
      Some(inode) => inode,
      None => {
          end_op();
          return -1;
      }
  };
  ilock(Some(ip));

  if readi(ip, 0, &mut elf as *mut Elfhdr as u64, 0, size_of::<Elfhdr>() as u32) != size_of::<Elfhdr>() as i32{
    bad(pagetable, sz, Some(ip));
    return -1;
  }

  if check_magic(&elf) == false {
    bad(pagetable, sz, Some(ip));
    return -1;
  }

  pagetable = proc_pagetable(p.unwrap());
  if pagetable.is_none() {
    bad(pagetable, sz, Some(ip));
    return -1;
  }

  off = elf.phoff as usize;
  for _ in 0..elf.phnum {
    off += size_of::<Proghdr>();
    if readi(ip, 0, &mut ph as *mut Proghdr as u64, off as u64, size_of::<Proghdr>() as u32) != size_of::<Proghdr>() as i32 {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
    if ph.typ != ELF_PROG_LOAD {
      continue;
    }
    if ph.memsz < ph.filesz {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
    if ph.vaddr + ph.memsz < ph.vaddr {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
    if (ph.vaddr % PGSIZE) != 0 {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
    let newsz = uvmalloc(pagetable.unwrap(), sz, ph.vaddr + ph.memsz, flags2perm(ph.flags as i32) as i32);
    if newsz == 0 {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
    sz = newsz;
    if loadseg(pagetable.unwrap(), ph.vaddr, ip, ph.off, ph.filesz) < 0 {
      bad(pagetable, sz, Some(ip));
      return -1;
    }
  }
  iunlockput(ip);
  end_op();
  
  p = myproc();
  let p = p.unwrap();
  let oldsz = p.sz;

  sz = PGROUNDUP(sz);
  let sz1 = uvmalloc(pagetable.unwrap(), sz, sz + (USERSTACK + 1) * PGSIZE, PTE_W as i32);
  if sz1 == 0 {
    bad(pagetable, sz, None);
    return -1;
  }
  sz = sz1;
  uvmclear(pagetable.unwrap(), sz - (USERSTACK + 1) * PGSIZE);
  sp = sz;
  stackbase = sp - (USERSTACK + 1) * PGSIZE;

  while !argv[argc as usize].is_null() {
    if argc >= MAXARG as u64 {
      bad(pagetable, sz, None);
      return -1;
    }
    sp -= size_of::<u64>() as u64;
    sp -= sp % 16;
    if sp < stackbase {
      bad(pagetable, sz, None);
      return -1;
    }
    if copyout(pagetable.unwrap(), sp, argv[argc as usize] as u64, size_of::<u64>() as u64) < 0 {
      bad(pagetable, sz, None);
      return -1;
    }
    ustack[argc as usize] = sp;
    argc += 1;
  }
  ustack[argc as usize] = 0;

  sp -= (argc + 1) * size_of::<u64>() as u64;
  sp -= sp % 16;

  if sp < stackbase {
    bad(pagetable, sz, None);
    return -1;
  }
  if copyout(pagetable.unwrap(), sp, ustack.as_ptr() as u64, (argc + 1) * size_of::<u64>() as u64) < 0 {
    bad(pagetable, sz, None);
    return -1;
  }

  unsafe { (*(p.trapframe)).a1 = sp; }

  last = match s.iter().rposition(|&c| c == b'/') {
    Some(pos) => &s[pos + 1..],
    None => s,
  };

  // safestrcpy();
  todo!("safestrcpy not implemented");

  oldpagetable = Some(p.pagetable);
  p.pagetable = pagetable.unwrap();
  p.sz = sz;
  unsafe {
    (*p.trapframe).epc = elf.entry;
    (*p.trapframe).sp = sp;
  }
  proc_freepagetable(oldpagetable.unwrap(), oldsz);

  argc as i32
}

fn loadseg(pagetable: PagetableT, va: u64, ip: *mut Inode, offset: u64, sz: u64) -> i32 {
  let mut n: u32;
  let mut pa: u64;

  for i in (0..sz).step_by(PGSIZE as usize) {
    pa = walkaddr(pagetable, va + i);
    if pa == 0 {
      panic("loadseg: address should exist".as_bytes());
    }

    n = if sz - i < PGSIZE {
      (sz - i) as u32
    } else {
      PGSIZE as u32
    };
    if readi(ip, 0, pa, offset + i, n) != n as i32 {
      return -1;
    }
  }
  0
}