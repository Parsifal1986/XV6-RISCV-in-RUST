use crate::proc;
use crate::spinlock::Spinlock;
use crate::riscv::{r_tp, PagetableT};
use crate::param::{NCPU, NPROC};

static mut cpus: [Cpu; NCPU as usize];

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
  pub fn new() -> Context {
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
  pub proc: Option<&'static mut Proc>,
  pub context: Context,
  pub noff: u64,
  pub intena: u64
}

impl Cpu {
  pub fn new() -> Cpu {
    Cpu {
      proc: None,
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

struct Proc {
  lock: Spinlock,
  state: Procstate,
  chan: u64,
  killed: bool,
  xstate: u64,
  pid: u64,

  parent: &'static mut Proc,

  kstatck: u64,
  sz: u64,
  pagetable: PagetableT,
  //trapframe: const *trapframe,
  context: Context,
  //file
  //inode
  name: [u8; 16]
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
    &mut cpus[id as usize]
  }
}