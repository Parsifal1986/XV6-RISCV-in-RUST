use core::arch::global_asm;
use core::ptr::{copy, null, null_mut};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::exec::kexec;
use crate::file::{filedup, fileclose, File, Inode};
use crate::fs::{fsinit, idup, iput, namei};
use crate::kalloc::{kalloc, kfree};
use crate::log::{begin_op, end_op};
use crate::memlayout::{kstack, KSTACKSIZE, TRAMPOLINE, TRAPFRAME};
use crate::param::{NCPU, NOFILE, NPROC, ROOTDEV};
use crate::printf::panic;
use crate::riscv::{intr_get, intr_off, intr_on, r_tp, wfi, PagetableT, MAKE_SATP, PGSIZE, PTE_R, PTE_W, PTE_X};
use crate::spinlock::{acquire, holding, initlock, pop_off, push_off, release, Spinlock};
use crate::string::safestrcpy;
use crate::trap::prepare_return;
use crate::vm::{copyin, copyout, kvmmap, mappages, uvmalloc, uvmcopy, uvmcreate, uvmdealloc, uvmfree, uvmunmap};

global_asm!(include_str!("swtch.S"));

extern "C" {
  // swtch.S
  fn swtch(old: *mut Context, new: *mut Context);

  // trampoline.S
  static trampoline: [u8; 0];
  static userret: [u8; 0];
}

// Saved registers for kernel context switches.
#[repr(C)]
pub struct Context {
  pub ra: u64,
  pub sp: u64,

  // callee-saved
  pub s0: u64,
  pub s1: u64,
  pub s2: u64,
  pub s3: u64,
  pub s4: u64,
  pub s5: u64,
  pub s6: u64,
  pub s7: u64,
  pub s8: u64,
  pub s9: u64,
  pub s10: u64,
  pub s11: u64,
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
      s11: 0,
    }
  }
}

// Per-CPU state.
pub struct Cpu {
  pub proc: *mut Proc,   // The process running on this cpu, or null.
  pub context: Context,  // swtch() here to enter scheduler().
  pub noff: i32,         // Depth of push_off() nesting.
  pub intena: bool,      // Were interrupts enabled before push_off()?
}

impl Cpu {
  pub const fn new() -> Self {
    Cpu {
      proc: null_mut(),
      context: Context::new(),
      noff: 0,
      intena: false,
    }
  }
}

// per-process data for the trap handling code in trampoline.S.
// sits in a page by itself just under the trampoline page in the
// user page table. not specially mapped in the kernel page table.
// uservec in trampoline.S saves user registers in the trapframe,
// then initializes registers from the trapframe's
// kernel_sp, kernel_hartid, kernel_satp, and jumps to kernel_trap.
// prepare_return() and userret in trampoline.S set up
// the trapframe's kernel_*, restore user registers from the
// trapframe, switch to the user page table, and enter user space.
// the trapframe includes callee-saved user registers like s0-s11 because the
// return-to-user path via prepare_return() doesn't return through
// the entire kernel call stack.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Trapframe {
  /*   0 */ pub kernel_satp: u64,   // kernel page table
  /*   8 */ pub kernel_sp: u64,     // top of process's kernel stack
  /*  16 */ pub kernel_trap: u64,   // usertrap()
  /*  24 */ pub epc: u64,           // saved user program counter
  /*  32 */ pub kernel_hartid: u64, // saved kernel tp
  /*  40 */ pub ra: u64,
  /*  48 */ pub sp: u64,
  /*  56 */ pub gp: u64,
  /*  64 */ pub tp: u64,
  /*  72 */ pub t0: u64,
  /*  80 */ pub t1: u64,
  /*  88 */ pub t2: u64,
  /*  96 */ pub s0: u64,
  /* 104 */ pub s1: u64,
  /* 112 */ pub a0: u64,
  /* 120 */ pub a1: u64,
  /* 128 */ pub a2: u64,
  /* 136 */ pub a3: u64,
  /* 144 */ pub a4: u64,
  /* 152 */ pub a5: u64,
  /* 160 */ pub a6: u64,
  /* 168 */ pub a7: u64,
  /* 176 */ pub s2: u64,
  /* 184 */ pub s3: u64,
  /* 192 */ pub s4: u64,
  /* 200 */ pub s5: u64,
  /* 208 */ pub s6: u64,
  /* 216 */ pub s7: u64,
  /* 224 */ pub s8: u64,
  /* 232 */ pub s9: u64,
  /* 240 */ pub s10: u64,
  /* 248 */ pub s11: u64,
  /* 256 */ pub t3: u64,
  /* 264 */ pub t4: u64,
  /* 272 */ pub t5: u64,
  /* 280 */ pub t6: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Procstate {
  UNUSED,
  USED,
  SLEEPING,
  RUNNABLE,
  RUNNING,
  ZOMBIE,
}

// Per-process state
pub struct Proc {
  pub lock: Spinlock,

  // p->lock must be held when using these:
  pub state: Procstate,        // Process state
  pub chan: *const u8,         // If non-null, sleeping on chan
  pub killed: bool,            // If true, have been killed
  pub xstate: i32,             // Exit status to be returned to parent's wait
  pub pid: i32,                // Process ID

  // WAIT_LOCK must be held when using this:
  pub parent: *mut Proc,       // Parent process

  // these are private to the process, so p->lock need not be held.
  pub kstack: u64,             // Virtual address of kernel stack
  pub sz: u64,                 // Size of process memory (bytes)
  pub pagetable: PagetableT,   // User page table
  pub trapframe: *mut Trapframe, // data page for trampoline.S
  pub context: Context,        // swtch() here to run process
  pub ofile: [*mut File; NOFILE], // Open files
  pub cwd: *mut Inode,         // Current directory
  pub name: [u8; 16],          // Process name (debugging)
}

impl Proc {
  pub const fn new() -> Self {
    Proc {
      lock: Spinlock::new(),
      state: Procstate::UNUSED,
      chan: null(),
      killed: false,
      xstate: 0,
      pid: 0,
      parent: null_mut(),
      kstack: 0,
      sz: 0,
      pagetable: null_mut(),
      trapframe: null_mut(),
      context: Context::new(),
      ofile: [null_mut(); NOFILE],
      cwd: null_mut(),
      name: [0; 16],
    }
  }
}

static mut CPUS: [Cpu; NCPU] = [const { Cpu::new() }; NCPU];

static mut PROC: [Proc; NPROC] = [const { Proc::new() }; NPROC];

static mut INITPROC: *mut Proc = null_mut();

static mut NEXTPID: i32 = 1;
static mut PID_LOCK: Spinlock = Spinlock::new();

// helps ensure that wakeups of wait()ing
// parents are not lost. helps obey the
// memory model when using p->parent.
// must be acquired before any p->lock.
static mut WAIT_LOCK: Spinlock = Spinlock::new();

// the i'th entry of the process table.
fn proc_at(i: usize) -> *mut Proc {
  unsafe { &raw mut PROC[i] }
}

// Allocate a page for each process's kernel stack.
// Map it high in memory, followed by an invalid
// guard page.
pub fn proc_mapstacks(kpgtbl: PagetableT) {
  for i in 0..NPROC {
    let va = kstack(i as u64);
    let mut a = 0;
    while a < KSTACKSIZE {
      let pa = kalloc();
      if pa.is_null() {
        panic("kalloc");
      }
      kvmmap(kpgtbl, va + a, pa as u64, PGSIZE, PTE_R | PTE_W);
      a += PGSIZE;
    }
  }
}

// initialize the proc table.
pub fn procinit() {
  initlock(&raw mut PID_LOCK, "nextpid");
  initlock(&raw mut WAIT_LOCK, "wait_lock");
  for i in 0..NPROC {
    let p = proc_at(i);
    unsafe {
      initlock(&raw mut (*p).lock, "proc");
      (*p).state = Procstate::UNUSED;
      (*p).kstack = kstack(i as u64);
    }
  }
}

// Must be called with interrupts disabled,
// to prevent race with process being moved
// to a different CPU.
pub fn cpuid() -> usize {
  r_tp() as usize
}

// Return this CPU's cpu struct.
// Interrupts must be disabled.
pub fn mycpu() -> *mut Cpu {
  let id = cpuid();
  unsafe { &raw mut CPUS[id] }
}

// Return the current struct proc *, or null if none.
pub fn myproc() -> *mut Proc {
  push_off();
  let c = mycpu();
  let p = unsafe { (*c).proc };
  pop_off();
  p
}

fn allocpid() -> i32 {
  unsafe {
    acquire(&raw mut PID_LOCK);
    let pid = NEXTPID;
    NEXTPID += 1;
    release(&raw mut PID_LOCK);
    pid
  }
}

// Look in the process table for an UNUSED proc.
// If found, initialize state required to run in the kernel,
// and return with p->lock held.
// If there are no free procs, or a memory allocation fails, return null.
fn allocproc() -> *mut Proc {
  let mut found: *mut Proc = null_mut();
  for i in 0..NPROC {
    let p = proc_at(i);
    unsafe {
      acquire(&raw mut (*p).lock);
      if (*p).state == Procstate::UNUSED {
        found = p;
        break;
      } else {
        release(&raw mut (*p).lock);
      }
    }
  }
  if found.is_null() {
    return null_mut();
  }

  let p = found;
  unsafe {
    (*p).pid = allocpid();
    (*p).state = Procstate::USED;

    // Allocate a trapframe page.
    (*p).trapframe = kalloc() as *mut Trapframe;
    if (*p).trapframe.is_null() {
      freeproc(p);
      release(&raw mut (*p).lock);
      return null_mut();
    }

    // An empty user page table.
    (*p).pagetable = proc_pagetable(p);
    if (*p).pagetable.is_null() {
      freeproc(p);
      release(&raw mut (*p).lock);
      return null_mut();
    }

    // Set up new context to start executing at forkret,
    // which returns to user space.
    (*p).context = Context::new();
    (*p).context.ra = forkret as *const () as u64;
    (*p).context.sp = (*p).kstack + KSTACKSIZE;
  }

  p
}

// free a proc structure and the data hanging from it,
// including user pages.
// p->lock must be held.
fn freeproc(p: *mut Proc) {
  unsafe {
    if !(*p).trapframe.is_null() {
      kfree((*p).trapframe as *mut u8);
    }
    (*p).trapframe = null_mut();
    if !(*p).pagetable.is_null() {
      proc_freepagetable((*p).pagetable, (*p).sz);
    }
    (*p).pagetable = null_mut();
    (*p).sz = 0;
    (*p).pid = 0;
    (*p).name[0] = 0;
    (*p).chan = null();
    (*p).killed = false;
    (*p).xstate = 0;
    (*p).state = Procstate::UNUSED;
  }
}

// Create a user page table for a given process, with no user memory,
// but with trampoline and trapframe pages.
pub fn proc_pagetable(p: *mut Proc) -> PagetableT {
  // An empty page table.
  let pagetable = uvmcreate();
  if pagetable.is_null() {
    return null_mut();
  }

  // map the trampoline code (for system call return)
  // at the highest user virtual address.
  // only the supervisor uses it, on the way
  // to/from user space, so not PTE_U.
  let tramp = unsafe { trampoline.as_ptr() as u64 };
  if mappages(pagetable, TRAMPOLINE, PGSIZE, tramp, PTE_R | PTE_X) < 0 {
    uvmfree(pagetable, 0);
    return null_mut();
  }

  // map the trapframe page just below the trampoline page, for
  // trampoline.S.
  let tf = unsafe { (*p).trapframe as u64 };
  if mappages(pagetable, TRAPFRAME, PGSIZE, tf, PTE_R | PTE_W) < 0 {
    uvmunmap(pagetable, TRAMPOLINE, 1, false);
    uvmfree(pagetable, 0);
    return null_mut();
  }

  pagetable
}

// Free a process's page table, and free the
// physical memory it refers to.
pub fn proc_freepagetable(pagetable: PagetableT, sz: u64) {
  uvmunmap(pagetable, TRAMPOLINE, 1, false);
  uvmunmap(pagetable, TRAPFRAME, 1, false);
  uvmfree(pagetable, sz);
}

// Set up first user process.
pub fn userinit() {
  let p = allocproc();
  if p.is_null() {
    panic("userinit");
  }
  unsafe {
    INITPROC = p;

    (*p).cwd = namei(b"/");

    (*p).state = Procstate::RUNNABLE;

    release(&raw mut (*p).lock);
  }
}

// Grow or shrink user memory by n bytes.
// Return 0 on success, -1 on failure.
pub fn growproc(n: i32) -> i32 {
  let p = myproc();
  unsafe {
    let mut sz = (*p).sz;
    if n > 0 {
      sz = uvmalloc((*p).pagetable, sz, sz + n as u64, PTE_W);
      if sz == 0 {
        return -1;
      }
    } else if n < 0 {
      sz = uvmdealloc((*p).pagetable, sz, sz.wrapping_add(n as i64 as u64));
    }
    (*p).sz = sz;
  }
  0
}

// Create a new process, copying the parent.
// Sets up child kernel stack to return as if from fork() system call.
pub fn kfork() -> i32 {
  let p = myproc();

  // Allocate process.
  let np = allocproc();
  if np.is_null() {
    return -1;
  }

  unsafe {
    // Copy user memory from parent to child.
    if uvmcopy((*p).pagetable, (*np).pagetable, (*p).sz) < 0 {
      freeproc(np);
      release(&raw mut (*np).lock);
      return -1;
    }
    (*np).sz = (*p).sz;

    // copy saved user registers.
    *(*np).trapframe = *(*p).trapframe;

    // Cause fork to return 0 in the child.
    (*(*np).trapframe).a0 = 0;

    // increment reference counts on open file descriptors.
    for i in 0..NOFILE {
      if !(*p).ofile[i].is_null() {
        (*np).ofile[i] = filedup((*p).ofile[i]);
      }
    }
    (*np).cwd = idup((*p).cwd);

    safestrcpy(&mut (*np).name, &(*p).name);

    let pid = (*np).pid;

    release(&raw mut (*np).lock);

    acquire(&raw mut WAIT_LOCK);
    (*np).parent = p;
    release(&raw mut WAIT_LOCK);

    acquire(&raw mut (*np).lock);
    (*np).state = Procstate::RUNNABLE;
    release(&raw mut (*np).lock);

    pid
  }
}

// Pass p's abandoned children to init.
// Caller must hold WAIT_LOCK.
fn reparent(p: *mut Proc) {
  for i in 0..NPROC {
    let pp = proc_at(i);
    unsafe {
      if (*pp).parent == p {
        (*pp).parent = INITPROC;
        wakeup(INITPROC as *const u8);
      }
    }
  }
}

// Exit the current process.  Does not return.
// An exited process remains in the zombie state
// until its parent calls wait().
pub fn kexit(status: i32) -> ! {
  let p = myproc();

  unsafe {
    if p == INITPROC {
      panic("init exiting");
    }

    // Close all open files.
    for fd in 0..NOFILE {
      if !(*p).ofile[fd].is_null() {
        let f = (*p).ofile[fd];
        fileclose(f);
        (*p).ofile[fd] = null_mut();
      }
    }

    begin_op();
    iput((*p).cwd);
    end_op();
    (*p).cwd = null_mut();

    acquire(&raw mut WAIT_LOCK);

    // Give any children to init.
    reparent(p);

    // Parent might be sleeping in wait().
    wakeup((*p).parent as *const u8);

    acquire(&raw mut (*p).lock);

    (*p).xstate = status;
    (*p).state = Procstate::ZOMBIE;

    release(&raw mut WAIT_LOCK);
  }

  // Jump into the scheduler, never to return.
  sched();
  panic("zombie exit");
}

// Wait for a child process to exit and return its pid.
// Return -1 if this process has no children.
pub fn kwait(addr: u64) -> i32 {
  let p = myproc();

  unsafe {
    acquire(&raw mut WAIT_LOCK);

    loop {
      // Scan through table looking for exited children.
      let mut havekids = false;
      for i in 0..NPROC {
        let pp = proc_at(i);
        if (*pp).parent == p {
          // make sure the child isn't still in exit() or swtch().
          acquire(&raw mut (*pp).lock);

          havekids = true;
          if (*pp).state == Procstate::ZOMBIE {
            // Found one.
            let pid = (*pp).pid;
            if addr != 0
              && copyout((*p).pagetable, addr, &raw const (*pp).xstate as *const u8, size_of::<i32>() as u64) < 0
            {
              release(&raw mut (*pp).lock);
              release(&raw mut WAIT_LOCK);
              return -1;
            }
            freeproc(pp);
            (*pp).parent = null_mut();
            release(&raw mut (*pp).lock);
            release(&raw mut WAIT_LOCK);
            return pid;
          }
          release(&raw mut (*pp).lock);
        }
      }

      // No point waiting if we don't have any children.
      if !havekids || killed(p) {
        release(&raw mut WAIT_LOCK);
        return -1;
      }

      // Wait for a child to exit.
      sleep(p as *const u8, &raw mut WAIT_LOCK); //DOC: wait-sleep
    }
  }
}

// Per-CPU process scheduler.
// Each CPU calls scheduler() after setting itself up.
// Scheduler never returns.  It loops, doing:
//  - choose a process to run.
//  - swtch to start running that process.
//  - eventually that process transfers control
//    via swtch back to the scheduler.
pub fn scheduler() -> ! {
  let c = mycpu();

  unsafe {
    (*c).proc = null_mut();
    loop {
      // The most recent process to run may have had interrupts
      // turned off; enable them to avoid a deadlock if all
      // processes are waiting. Then turn them back off
      // to avoid a possible race between an interrupt
      // and wfi.
      intr_on();
      intr_off();

      let mut found = false;
      for i in 0..NPROC {
        let p = proc_at(i);
        acquire(&raw mut (*p).lock);
        if (*p).state == Procstate::RUNNABLE {
          // Switch to chosen process.  It is the process's job
          // to release its lock and then reacquire it
          // before jumping back to us.
          (*p).state = Procstate::RUNNING;
          (*c).proc = p;
          swtch(&raw mut (*c).context, &raw mut (*p).context);

          // Don't re-enable interrupts on release.
          (*mycpu()).intena = false;

          // Process is done running for now.
          // It should have changed its p->state before coming back.
          (*c).proc = null_mut();
          found = true;
        }
        release(&raw mut (*p).lock);
      }
      if !found {
        // nothing to run; stop running on this core until an interrupt.
        wfi();
      }
    }
  }
}

// Switch to scheduler.  Must hold only p->lock
// and have changed proc->state. Saves and restores
// intena because intena is a property of this
// kernel thread, not this CPU. It should
// be proc->intena and proc->noff, but that would
// break in the few places where a lock is held but
// there's no process.
pub fn sched() {
  let p = myproc();

  unsafe {
    if !holding(&raw mut (*p).lock) {
      panic("sched p->lock");
    }
    if (*mycpu()).noff != 1 {
      panic("sched locks");
    }
    if (*p).state == Procstate::RUNNING {
      panic("sched RUNNING");
    }
    if intr_get() {
      panic("sched interruptible");
    }

    let intena = (*mycpu()).intena;
    swtch(&raw mut (*p).context, &raw mut (*mycpu()).context);
    (*mycpu()).intena = intena;
  }
}

// Give up the CPU for one scheduling round.
pub fn yieldcpu() {
  let p = myproc();
  unsafe {
    acquire(&raw mut (*p).lock);
    (*p).state = Procstate::RUNNABLE;
    sched();
    release(&raw mut (*p).lock);
  }
}

static FIRST: AtomicBool = AtomicBool::new(true);

// A fork child's very first scheduling by scheduler()
// will swtch to forkret.
extern "C" fn forkret() -> ! {
  let p = myproc();

  unsafe {
    // Still holding p->lock from scheduler.
    release(&raw mut (*p).lock);

    if FIRST.load(Ordering::Acquire) {
      // File system initialization must be run in the context of a
      // regular process (e.g., because it calls sleep), and thus cannot
      // be run from main().
      fsinit(ROOTDEV);

      // ensure other cores see FIRST == false.
      FIRST.store(false, Ordering::Release);

      // We can invoke kexec() now that file system is initialized.
      // Put the return value (argc) of kexec into a0.
      let argv: [*const u8; 2] = [b"/init\0".as_ptr(), null()];
      let r = kexec(b"/init", &argv);
      if r == -1 {
        panic("exec");
      }
      (*(*p).trapframe).a0 = r as u64;
    }

    // return to user space, mimicing usertrap()'s return.
    prepare_return();
    let satp = MAKE_SATP((*p).pagetable as u64);
    let trampoline_userret = TRAMPOLINE + (userret.as_ptr() as u64 - trampoline.as_ptr() as u64);
    let f: extern "C" fn(u64) -> ! = core::mem::transmute(trampoline_userret as usize);
    f(satp)
  }
}

// Sleep on channel chan, releasing condition lock lk.
// Re-acquires lk when awakened.
pub fn sleep(chan: *const u8, lk: *mut Spinlock) {
  let p = myproc();

  unsafe {
    // Must acquire p->lock in order to
    // change p->state and then call sched.
    // Once we hold p->lock, we can be
    // guaranteed that we won't miss any wakeup
    // (wakeup locks p->lock),
    // so it's okay to release lk.

    acquire(&raw mut (*p).lock); //DOC: sleeplock1
    release(lk);

    // Go to sleep.
    (*p).chan = chan;
    (*p).state = Procstate::SLEEPING;

    sched();

    // Tidy up.
    (*p).chan = null();

    // Reacquire original lock.
    release(&raw mut (*p).lock);
    acquire(lk);
  }
}

// Wake up all processes sleeping on channel chan.
// Caller should hold the condition lock.
pub fn wakeup(chan: *const u8) {
  let me = myproc();
  for i in 0..NPROC {
    let p = proc_at(i);
    if p != me {
      unsafe {
        acquire(&raw mut (*p).lock);
        if (*p).state == Procstate::SLEEPING && (*p).chan == chan {
          (*p).state = Procstate::RUNNABLE;
        }
        release(&raw mut (*p).lock);
      }
    }
  }
}

// Kill the process with the given pid.
// The victim won't exit until it tries to return
// to user space (see usertrap() in trap.rs).
pub fn kkill(pid: i32) -> i32 {
  for i in 0..NPROC {
    let p = proc_at(i);
    unsafe {
      acquire(&raw mut (*p).lock);
      if (*p).pid == pid {
        (*p).killed = true;
        if (*p).state == Procstate::SLEEPING {
          // Wake process from sleep().
          (*p).state = Procstate::RUNNABLE;
        }
        release(&raw mut (*p).lock);
        return 0;
      }
      release(&raw mut (*p).lock);
    }
  }
  -1
}

pub fn setkilled(p: *mut Proc) {
  unsafe {
    acquire(&raw mut (*p).lock);
    (*p).killed = true;
    release(&raw mut (*p).lock);
  }
}

pub fn killed(p: *mut Proc) -> bool {
  unsafe {
    acquire(&raw mut (*p).lock);
    let k = (*p).killed;
    release(&raw mut (*p).lock);
    k
  }
}

// Copy to either a user address, or kernel address,
// depending on user_dst.
// Returns 0 on success, -1 on error.
pub fn either_copyout(user_dst: bool, dst: u64, src: *const u8, len: u64) -> i32 {
  let p = myproc();
  if user_dst {
    unsafe { copyout((*p).pagetable, dst, src, len) }
  } else {
    unsafe { copy(src, dst as *mut u8, len as usize) };
    0
  }
}

// Copy from either a user address, or kernel address,
// depending on user_src.
// Returns 0 on success, -1 on error.
pub fn either_copyin(dst: *mut u8, user_src: bool, src: u64, len: u64) -> i32 {
  let p = myproc();
  if user_src {
    unsafe { copyin((*p).pagetable, dst, src, len) }
  } else {
    unsafe { copy(src as *const u8, dst, len as usize) };
    0
  }
}

// Print a process listing to console.  For debugging.
// Runs when user types ^P on console.
// No lock to avoid wedging a stuck machine further.
pub fn procdump() {
  printf!("\n");
  for i in 0..NPROC {
    let p = proc_at(i);
    unsafe {
      let state = match (*p).state {
        Procstate::UNUSED => continue,
        Procstate::USED => "used",
        Procstate::SLEEPING => "sleep ",
        Procstate::RUNNABLE => "runble",
        Procstate::RUNNING => "run   ",
        Procstate::ZOMBIE => "zombie",
      };
      let name = crate::string::cstr(&(*p).name);
      printf!("{} {} {}\n", (*p).pid, state, core::str::from_utf8(name).unwrap_or("???"));
    }
  }
}
