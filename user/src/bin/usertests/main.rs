//
// Tests xv6 system calls.  usertests without arguments runs them all
// and usertests <name> runs <name> test. The test runner creates for
// each test a process and based on the exit status of the process,
// the test runner reports "OK" or "FAILED".  Some tests result in
// kernel printing usertrap messages, which can be ignored if test
// prints "OK".
//

#![no_std]
#![no_main]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use core::arch::asm;

use user::*;

mod part1;
mod part2;
mod slow;

// from kernel/src/param.rs, fs.rs, riscv.rs and memlayout.rs.
pub const MAXOPBLOCKS: usize = 10;
pub const BSIZE: usize = 1024;
pub const NDIRECT: usize = 12;
pub const NINDIRECT: usize = BSIZE / 4;
pub const MAXFILE: usize = NDIRECT + NINDIRECT;
pub const NINODE: usize = 50;
pub const NOFILE: usize = 16;
pub const MAXVA: u64 = 1 << (9 + 9 + 9 + 12 - 1);
pub const TRAMPOLINE: u64 = MAXVA - PGSIZE as u64;
pub const TRAPFRAME: u64 = TRAMPOLINE - PGSIZE as u64;
pub const KERNBASE: u64 = 0x8000_0000;
pub const PHYSTOP: u64 = KERNBASE + 128 * 1024 * 1024;

pub const BUFSZ: usize = (MAXOPBLOCKS + 2) * BSIZE;

pub static mut BUF: [u8; BUFSZ] = [0; BUFSZ];

// the global scratch buffer shared by many tests.
#[allow(clippy::mut_from_ref)]
pub fn buf() -> &'static mut [u8; BUFSZ] {
  unsafe { &mut *(&raw mut BUF) }
}

// read the stack pointer.
#[inline(always)]
pub fn r_sp() -> u64 {
  let x: u64;
  unsafe {
    asm!("mv {0}, sp", out(reg) x);
  }
  x
}

//
// Section with tests that run fairly quickly.  Use -q if you want to
// run just those.  Without -q usertests also runs the ones that take a
// fair amount of time.
//

pub struct Test {
  f: fn(&str),
  s: &'static str,
}

const fn t(f: fn(&str), s: &'static str) -> Test {
  Test { f, s }
}

static QUICKTESTS: &[Test] = &[
  t(part1::copyin, "copyin"),
  t(part1::copyout, "copyout"),
  t(part1::copyinstr1, "copyinstr1"),
  t(part1::copyinstr2, "copyinstr2"),
  t(part1::copyinstr3, "copyinstr3"),
  t(part1::rwsbrk, "rwsbrk"),
  t(part1::truncate1, "truncate1"),
  t(part1::truncate2, "truncate2"),
  t(part1::truncate3, "truncate3"),
  t(part1::openiputtest, "openiput"),
  t(part1::exitiputtest, "exitiput"),
  t(part1::iputtest, "iput"),
  t(part1::opentest, "opentest"),
  t(part1::writetest, "writetest"),
  t(part1::writebig, "writebig"),
  t(part1::createtest, "createtest"),
  t(part1::dirtest, "dirtest"),
  t(part1::exectest, "exectest"),
  t(part1::pipe1, "pipe1"),
  t(part1::killstatus, "killstatus"),
  t(part1::preempt, "preempt"),
  t(part1::exitwait, "exitwait"),
  t(part1::reparent, "reparent"),
  t(part1::twochildren, "twochildren"),
  t(part1::forkfork, "forkfork"),
  t(part1::forkforkfork, "forkforkfork"),
  t(part1::reparent2, "reparent2"),
  t(part1::mem, "mem"),
  t(part1::sharedfd, "sharedfd"),
  t(part1::fourfiles, "fourfiles"),
  t(part1::createdelete, "createdelete"),
  t(part1::unlinkread, "unlinkread"),
  t(part1::linktest, "linktest"),
  t(part1::concreate, "concreate"),
  t(part1::linkunlink, "linkunlink"),
  t(part2::subdir, "subdir"),
  t(part2::bigwrite, "bigwrite"),
  t(part2::bigfile, "bigfile"),
  t(part2::fourteen, "fourteen"),
  t(part2::rmdot, "rmdot"),
  t(part2::dirfile, "dirfile"),
  t(part2::iref, "iref"),
  t(part2::forktest, "forktest"),
  t(part2::sbrkbasic, "sbrkbasic"),
  t(part2::sbrkmuch, "sbrkmuch"),
  t(part2::kernmem, "kernmem"),
  t(part2::MAXVAplus, "MAXVAplus"),
  t(part2::sbrkfail, "sbrkfail"),
  t(part2::sbrkarg, "sbrkarg"),
  t(part2::validatetest, "validatetest"),
  t(part2::bsstest, "bsstest"),
  t(part2::bigargtest, "bigargtest"),
  t(part2::argptest, "argptest"),
  t(part2::stacktest, "stacktest"),
  t(part2::nowrite, "nowrite"),
  t(part2::pgbug, "pgbug"),
  t(part2::sbrkbugs, "sbrkbugs"),
  t(part2::sbrklast, "sbrklast"),
  t(part2::sbrk8000, "sbrk8000"),
  t(part2::badarg, "badarg"),
  t(part2::lazy_alloc, "lazy_alloc"),
  t(part2::lazy_unmap, "lazy_unmap"),
  t(part2::lazy_copy, "lazy_copy"),
];

//
// Section with tests that take a fair bit of time
//

static SLOWTESTS: &[Test] = &[
  t(slow::bigdir, "bigdir"),
  t(slow::manywrites, "manywrites"),
  t(slow::badwrite, "badwrite"),
  t(slow::execout, "execout"),
  t(slow::diskfull, "diskfull"),
  t(slow::outofinodes, "outofinodes"),
];

//
// drive tests
//

// run each test in its own process. run returns true if child's exit()
// indicates success.
fn run(f: fn(&str), s: &str) -> bool {
  let mut xstatus = 0;

  printf!("test {}: ", s);
  let pid = fork();
  if pid < 0 {
    printf!("runtest: fork error\n");
    exit(1);
  }
  if pid == 0 {
    f(s);
    exit(0);
  } else {
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      printf!("FAILED\n");
    } else {
      printf!("OK\n");
    }
    xstatus == 0
  }
}

fn runtests(tests: &[Test], justone: Option<&str>, continuous: i32) -> i32 {
  let mut ntests = 0;
  for t in tests {
    if justone.is_none() || justone == Some(t.s) {
      ntests += 1;
      if !run(t.f, t.s) && continuous != 2 {
        printf!("SOME TESTS FAILED\n");
        return -1;
      }
    }
  }
  ntests
}

// use sbrk() to count how many free physical memory pages there are.
pub fn countfree() -> i32 {
  let mut n = 0;
  let sz0 = sbrk(0) as u64;
  loop {
    let a = sbrk(PGSIZE as i32);
    if a == SBRK_ERROR {
      break;
    }
    n += 1;
  }
  sbrk(-((sbrk(0) as u64 - sz0) as i32));
  n
}

fn drivetests(quick: bool, continuous: i32, justone: Option<&str>) -> i32 {
  loop {
    printf!("usertests starting\n");
    let free0 = countfree();
    let mut ntests = 0;
    let n = runtests(QUICKTESTS, justone, continuous);
    if n < 0 {
      if continuous != 2 {
        return 1;
      }
    } else {
      ntests += n;
    }
    if !quick {
      if justone.is_none() {
        printf!("usertests slow tests starting\n");
      }
      let n = runtests(SLOWTESTS, justone, continuous);
      if n < 0 {
        if continuous != 2 {
          return 1;
        }
      } else {
        ntests += n;
      }
    }
    let free1 = countfree();
    if free1 < free0 {
      printf!("FAILED -- lost some free pages {} (out of {})\n", free1, free0);
      if continuous != 2 {
        return 1;
      }
    }
    if justone.is_some() && ntests == 0 {
      printf!("NO TESTS EXECUTED\n");
      return 1;
    }
    if continuous == 0 {
      break;
    }
  }
  0
}

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  let mut continuous = 0;
  let mut quick = false;
  let mut justone = None;

  if args.len() == 2 && args[1] == "-q" {
    quick = true;
  } else if args.len() == 2 && args[1] == "-c" {
    continuous = 1;
  } else if args.len() == 2 && args[1] == "-C" {
    continuous = 2;
  } else if args.len() == 2 && !args[1].starts_with('-') {
    justone = Some(args[1]);
  } else if args.len() > 1 {
    printf!("Usage: usertests [-c] [-C] [-q] [testname]\n");
    exit(1);
  }
  if drivetests(quick, continuous, justone) != 0 {
    exit(1);
  }
  printf!("ALL TESTS PASSED\n");
  0
}
