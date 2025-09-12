use crate::file::{Inode, File};
use crate::spinlock::{acquire, pop_off, push_off, release, Spinlock};
use crate::riscv::{r_tp, PagetableT};
use crate::param::{NCPU, NOFILE, NPROC};

static mut CPUS: [Cpu; NCPU as usize] = [const { Cpu::new() }; NCPU as usize];

pub struct Context {
  ra: u64,
  sp: u64,
  
  s0: u64,
  s1: u64,
  s2: u64,
  s3: u64,
  s4: u64,
  s5: u64,
  s6: u64,
  s7: u64,
  s8: u64,
  s9: u64,
  s10: u64,
  s11: u64
}

impl Context {
  pub const fn new() -> Self {
    Context {
      ra: 0,
      sp: 0,
      s0: 0,
      s1: 0,
      s2: 0,
      s3: 0,
      s4: 0,
      s5: 0,
      s6: 0,
      s7: 0,
      s8: 0,
      s9: 0,
      s10: 0,
      s11: 0
    }
  }
}

pub struct Cpu {
  pub proc: *mut Proc,
  pub context: Context,
  pub noff: u64,
  pub intena: u64
}

impl Cpu {
  pub const fn new() -> Self {
    Cpu {
      proc: core::ptr::null_mut(),
      context: Context::new(),
      noff: 0,
      intena: 0
    }
  }
}

enum Procstate {
  UNUSED,
  USED,
  SLEEPING,
  RUNNABLE,
  RUNNING,
  ZOMBIE
}

#[repr(C)]
pub struct Trapframe {
    /*   0 */ pub(crate) kernel_satp: u64,
    /*   8 */ pub(crate) kernel_sp: u64,
    /*  16 */ pub(crate) kernel_trap: u64,
    /*  24 */ pub(crate) epc: u64,
    /*  32 */ pub(crate) kernel_hartid: u64,
    /*  40 */ pub(crate) ra: u64,
    /*  48 */ pub(crate) sp: u64,
    /*  56 */ pub(crate) gp: u64,
    /*  64 */ pub(crate) tp: u64,
    /*  72 */ pub(crate) t0: u64,
    /*  80 */ pub(crate) t1: u64,
    /*  88 */ pub(crate) t2: u64,
    /*  96 */ pub(crate) s0: u64,
    /* 104 */ pub(crate) s1: u64,
    /* 112 */ pub(crate) a0: u64,
    /* 120 */ pub(crate) a1: u64,
    /* 128 */ pub(crate) a2: u64,
    /* 136 */ pub(crate) a3: u64,
    /* 144 */ pub(crate) a4: u64,
    /* 152 */ pub(crate) a5: u64,
    /* 160 */ pub(crate) a6: u64,
    /* 168 */ pub(crate) a7: u64,
    /* 176 */ pub(crate) s2: u64,
    /* 184 */ pub(crate) s3: u64,
    /* 192 */ pub(crate) s4: u64,
    /* 200 */ pub(crate) s5: u64,
    /* 208 */ pub(crate) s6: u64,
    /* 216 */ pub(crate) s7: u64,
    /* 224 */ pub(crate) s8: u64,
    /* 232 */ pub(crate) s9: u64,
    /* 240 */ pub(crate) s10: u64,
    /* 248 */ pub(crate) s11: u64,
    /* 256 */ pub(crate) t3: u64,
    /* 264 */ pub(crate) t4: u64,
    /* 272 */ pub(crate) t5: u64,
    /* 280 */ pub(crate) t6: u64,
}

pub struct Proc {
  lock: Spinlock,
  state: Procstate,
  chan: u64,
  killed: bool,
  xstate: u64,
  pub(crate) pid: u64,

  parent: &'static mut Proc,

  pub(crate) kstack: u64,
  pub(crate) sz: u64,
  pub(crate) pagetable: PagetableT,
  pub(crate) trapframe: *mut Trapframe,
  context: Context,
  pub(crate) ofile: [Option<*mut File>; NOFILE as usize],
  cwd: *mut Inode,
  pub(crate) name: [u8; 16],
}

pub fn proc_mapstacks(kpgtbl: PagetableT) {
  todo!();
}

pub fn cpuid() -> u64 {
  r_tp()
}

pub fn mycpu() -> &'static mut Cpu {
  let id = cpuid();
  unsafe {
    &mut CPUS[id as usize]
  }
}

pub fn myproc() -> Option<&'static mut Proc> {
  push_off();
  let c = mycpu();
  let p: Option<&'static mut Proc>;
  if c.proc.is_null() {
    p = None;
  } else {
    p = Some(unsafe { &mut *c.proc });
  }
  pop_off();
  p
}

pub fn allocpid() -> i32 {
  todo!()
}

fn allocproc() -> Option<*mut Proc> {
  todo!()
}

fn freeproc() {
  todo!()
}

pub fn proc_pagetable(p: *mut Proc) -> Option<PagetableT> {
  todo!()
}

pub fn proc_freepagetable(pagetable: PagetableT, sz: u64) {
  todo!()
}

pub fn sleep(chan: *mut u8, lk: &mut Spinlock) {
  todo!();
}

pub fn either_copyout(user_dst: i32, dst: *const u8, src: *const u8, len: u64) -> i32 {
  todo!()
}

pub fn either_copyin(dst: *const u8, user_src: i32, src: *const u8, len: u64) -> i32 {
  todo!()
}

pub fn setkilled(p: &mut Proc) {
  acquire(&mut p.lock);
  p.killed = true;
  release(&mut p.lock);
}

pub fn killed(p: &mut Proc) -> bool {
  let k: bool;

  acquire(&mut p.lock);
  k = p.killed;
  release(&mut p.lock);
  
  k
}

pub fn kexit(status: i32) {
  todo!()
}

pub fn yieldcpu() {
  todo!()
}

pub fn wakeup(chan: *mut u8) {
  todo!()
}

pub fn kfork() -> i32 {
  todo!()
}

pub fn kwait(addr: u64) -> i32 {
  todo!()
}

pub fn kkill(pid: i32) -> i32 {
  todo!()
}

pub fn growproc(n: i32) -> i32 {
  todo!()
}