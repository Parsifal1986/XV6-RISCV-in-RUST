// usertests: see main.rs.
// ported from xv6's user/usertests.c.

use core::hint::black_box;
use core::ptr::{null, read_volatile, write_volatile};

use user::*;

use crate::*;

pub fn subdir(s: &str) {
  unlink("ff");
  if mkdir("dd") != 0 {
    printf!("{}: mkdir dd failed\n", s);
    exit(1);
  }

  let fd = open("dd/ff", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: create dd/ff failed\n", s);
    exit(1);
  }
  write(fd, b"ff");
  close(fd);

  if unlink("dd") >= 0 {
    printf!("{}: unlink dd (non-empty dir) succeeded!\n", s);
    exit(1);
  }

  if mkdir("/dd/dd") != 0 {
    printf!("{}: subdir mkdir dd/dd failed\n", s);
    exit(1);
  }

  let fd = open("dd/dd/ff", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: create dd/dd/ff failed\n", s);
    exit(1);
  }
  write(fd, b"FF");
  close(fd);

  let fd = open("dd/dd/../ff", 0);
  if fd < 0 {
    printf!("{}: open dd/dd/../ff failed\n", s);
    exit(1);
  }
  let cc = read(fd, buf());
  if cc != 2 || buf()[0] != b'f' {
    printf!("{}: dd/dd/../ff wrong content\n", s);
    exit(1);
  }
  close(fd);

  if link("dd/dd/ff", "dd/dd/ffff") != 0 {
    printf!("{}: link dd/dd/ff dd/dd/ffff failed\n", s);
    exit(1);
  }

  if unlink("dd/dd/ff") != 0 {
    printf!("{}: unlink dd/dd/ff failed\n", s);
    exit(1);
  }
  if open("dd/dd/ff", O_RDONLY) >= 0 {
    printf!("{}: open (unlinked) dd/dd/ff succeeded\n", s);
    exit(1);
  }

  if chdir("dd") != 0 {
    printf!("{}: chdir dd failed\n", s);
    exit(1);
  }
  if chdir("dd/../../dd") != 0 {
    printf!("{}: chdir dd/../../dd failed\n", s);
    exit(1);
  }
  if chdir("dd/../../../dd") != 0 {
    printf!("{}: chdir dd/../../../dd failed\n", s);
    exit(1);
  }
  if chdir("./..") != 0 {
    printf!("{}: chdir ./.. failed\n", s);
    exit(1);
  }

  let fd = open("dd/dd/ffff", 0);
  if fd < 0 {
    printf!("{}: open dd/dd/ffff failed\n", s);
    exit(1);
  }
  if read(fd, buf()) != 2 {
    printf!("{}: read dd/dd/ffff wrong len\n", s);
    exit(1);
  }
  close(fd);

  if open("dd/dd/ff", O_RDONLY) >= 0 {
    printf!("{}: open (unlinked) dd/dd/ff succeeded!\n", s);
    exit(1);
  }

  if open("dd/ff/ff", O_CREATE | O_RDWR) >= 0 {
    printf!("{}: create dd/ff/ff succeeded!\n", s);
    exit(1);
  }
  if open("dd/xx/ff", O_CREATE | O_RDWR) >= 0 {
    printf!("{}: create dd/xx/ff succeeded!\n", s);
    exit(1);
  }
  if open("dd", O_CREATE) >= 0 {
    printf!("{}: create dd succeeded!\n", s);
    exit(1);
  }
  if open("dd", O_RDWR) >= 0 {
    printf!("{}: open dd rdwr succeeded!\n", s);
    exit(1);
  }
  if open("dd", O_WRONLY) >= 0 {
    printf!("{}: open dd wronly succeeded!\n", s);
    exit(1);
  }
  if link("dd/ff/ff", "dd/dd/xx") == 0 {
    printf!("{}: link dd/ff/ff dd/dd/xx succeeded!\n", s);
    exit(1);
  }
  if link("dd/xx/ff", "dd/dd/xx") == 0 {
    printf!("{}: link dd/xx/ff dd/dd/xx succeeded!\n", s);
    exit(1);
  }
  if link("dd/ff", "dd/dd/ffff") == 0 {
    printf!("{}: link dd/ff dd/dd/ffff succeeded!\n", s);
    exit(1);
  }
  if mkdir("dd/ff/ff") == 0 {
    printf!("{}: mkdir dd/ff/ff succeeded!\n", s);
    exit(1);
  }
  if mkdir("dd/xx/ff") == 0 {
    printf!("{}: mkdir dd/xx/ff succeeded!\n", s);
    exit(1);
  }
  if mkdir("dd/dd/ffff") == 0 {
    printf!("{}: mkdir dd/dd/ffff succeeded!\n", s);
    exit(1);
  }
  if unlink("dd/xx/ff") == 0 {
    printf!("{}: unlink dd/xx/ff succeeded!\n", s);
    exit(1);
  }
  if unlink("dd/ff/ff") == 0 {
    printf!("{}: unlink dd/ff/ff succeeded!\n", s);
    exit(1);
  }
  if chdir("dd/ff") == 0 {
    printf!("{}: chdir dd/ff succeeded!\n", s);
    exit(1);
  }
  if chdir("dd/xx") == 0 {
    printf!("{}: chdir dd/xx succeeded!\n", s);
    exit(1);
  }

  if unlink("dd/dd/ffff") != 0 {
    printf!("{}: unlink dd/dd/ff failed\n", s);
    exit(1);
  }
  if unlink("dd/ff") != 0 {
    printf!("{}: unlink dd/ff failed\n", s);
    exit(1);
  }
  if unlink("dd") == 0 {
    printf!("{}: unlink non-empty dd succeeded!\n", s);
    exit(1);
  }
  if unlink("dd/dd") < 0 {
    printf!("{}: unlink dd/dd failed\n", s);
    exit(1);
  }
  if unlink("dd") < 0 {
    printf!("{}: unlink dd failed\n", s);
    exit(1);
  }
}

// test writes that are larger than the log.
pub fn bigwrite(s: &str) {
  unlink("bigwrite");
  let mut sz = 499;
  while sz < (MAXOPBLOCKS + 2) * BSIZE {
    let fd = open("bigwrite", O_CREATE | O_RDWR);
    if fd < 0 {
      printf!("{}: cannot create bigwrite\n", s);
      exit(1);
    }
    for _ in 0..2 {
      let cc = write(fd, &buf()[..sz]);
      if cc != sz as i32 {
        printf!("{}: write({}) ret {}\n", s, sz, cc);
        exit(1);
      }
    }
    close(fd);
    unlink("bigwrite");
    sz += 471;
  }
}

pub fn bigfile(s: &str) {
  const N: usize = 20;
  const SZ: usize = 600;

  unlink("bigfile.dat");
  let fd = open("bigfile.dat", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: cannot create bigfile", s);
    exit(1);
  }
  for i in 0..N {
    buf()[..SZ].fill(i as u8);
    if write(fd, &buf()[..SZ]) != SZ as i32 {
      printf!("{}: write bigfile failed\n", s);
      exit(1);
    }
  }
  close(fd);

  let fd = open("bigfile.dat", 0);
  if fd < 0 {
    printf!("{}: cannot open bigfile\n", s);
    exit(1);
  }
  let mut total = 0;
  let mut i = 0;
  loop {
    let cc = read(fd, &mut buf()[..SZ / 2]);
    if cc < 0 {
      printf!("{}: read bigfile failed\n", s);
      exit(1);
    }
    if cc == 0 {
      break;
    }
    if cc != (SZ / 2) as i32 {
      printf!("{}: short read bigfile\n", s);
      exit(1);
    }
    if buf()[0] as usize != i / 2 || buf()[SZ / 2 - 1] as usize != i / 2 {
      printf!("{}: read bigfile wrong data\n", s);
      exit(1);
    }
    total += cc;
    i += 1;
  }
  close(fd);
  if total != (N * SZ) as i32 {
    printf!("{}: read bigfile wrong total\n", s);
    exit(1);
  }
  unlink("bigfile.dat");
}

pub fn fourteen(s: &str) {
  // DIRSIZ is 14.

  if mkdir("12345678901234") != 0 {
    printf!("{}: mkdir 12345678901234 failed\n", s);
    exit(1);
  }
  if mkdir("12345678901234/123456789012345") != 0 {
    printf!("{}: mkdir 12345678901234/123456789012345 failed\n", s);
    exit(1);
  }
  let fd = open("123456789012345/123456789012345/123456789012345", O_CREATE);
  if fd < 0 {
    printf!("{}: create 123456789012345/123456789012345/123456789012345 failed\n", s);
    exit(1);
  }
  close(fd);
  let fd = open("12345678901234/12345678901234/12345678901234", 0);
  if fd < 0 {
    printf!("{}: open 12345678901234/12345678901234/12345678901234 failed\n", s);
    exit(1);
  }
  close(fd);

  if mkdir("12345678901234/12345678901234") == 0 {
    printf!("{}: mkdir 12345678901234/12345678901234 succeeded!\n", s);
    exit(1);
  }
  if mkdir("123456789012345/12345678901234") == 0 {
    printf!("{}: mkdir 12345678901234/123456789012345 succeeded!\n", s);
    exit(1);
  }

  // clean up
  unlink("123456789012345/12345678901234");
  unlink("12345678901234/12345678901234");
  unlink("12345678901234/12345678901234/12345678901234");
  unlink("123456789012345/123456789012345/123456789012345");
  unlink("12345678901234/123456789012345");
  unlink("12345678901234");
}

pub fn rmdot(s: &str) {
  if mkdir("dots") != 0 {
    printf!("{}: mkdir dots failed\n", s);
    exit(1);
  }
  if chdir("dots") != 0 {
    printf!("{}: chdir dots failed\n", s);
    exit(1);
  }
  if unlink(".") == 0 {
    printf!("{}: rm . worked!\n", s);
    exit(1);
  }
  if unlink("..") == 0 {
    printf!("{}: rm .. worked!\n", s);
    exit(1);
  }
  if chdir("/") != 0 {
    printf!("{}: chdir / failed\n", s);
    exit(1);
  }
  if unlink("dots/.") == 0 {
    printf!("{}: unlink dots/. worked!\n", s);
    exit(1);
  }
  if unlink("dots/..") == 0 {
    printf!("{}: unlink dots/.. worked!\n", s);
    exit(1);
  }
  if unlink("dots") != 0 {
    printf!("{}: unlink dots failed!\n", s);
    exit(1);
  }
}

pub fn dirfile(s: &str) {
  let fd = open("dirfile", O_CREATE);
  if fd < 0 {
    printf!("{}: create dirfile failed\n", s);
    exit(1);
  }
  close(fd);
  if chdir("dirfile") == 0 {
    printf!("{}: chdir dirfile succeeded!\n", s);
    exit(1);
  }
  let fd = open("dirfile/xx", 0);
  if fd >= 0 {
    printf!("{}: create dirfile/xx succeeded!\n", s);
    exit(1);
  }
  let fd = open("dirfile/xx", O_CREATE);
  if fd >= 0 {
    printf!("{}: create dirfile/xx succeeded!\n", s);
    exit(1);
  }
  if mkdir("dirfile/xx") == 0 {
    printf!("{}: mkdir dirfile/xx succeeded!\n", s);
    exit(1);
  }
  if unlink("dirfile/xx") == 0 {
    printf!("{}: unlink dirfile/xx succeeded!\n", s);
    exit(1);
  }
  if link("README", "dirfile/xx") == 0 {
    printf!("{}: link to dirfile/xx succeeded!\n", s);
    exit(1);
  }
  if unlink("dirfile") != 0 {
    printf!("{}: unlink dirfile failed!\n", s);
    exit(1);
  }

  let fd = open(".", O_RDWR);
  if fd >= 0 {
    printf!("{}: open . for writing succeeded!\n", s);
    exit(1);
  }
  let fd = open(".", 0);
  if write(fd, b"x") > 0 {
    printf!("{}: write . succeeded!\n", s);
    exit(1);
  }
  close(fd);
}

// test that iput() is called at the end of _namei().
// also tests empty file names.
pub fn iref(s: &str) {
  for _ in 0..NINODE + 1 {
    if mkdir("irefd") != 0 {
      printf!("{}: mkdir irefd failed\n", s);
      exit(1);
    }
    if chdir("irefd") != 0 {
      printf!("{}: chdir irefd failed\n", s);
      exit(1);
    }

    mkdir("");
    link("README", "");
    let fd = open("", O_CREATE);
    if fd >= 0 {
      close(fd);
    }
    let fd = open("xx", O_CREATE);
    if fd >= 0 {
      close(fd);
    }
    unlink("xx");
  }

  // clean up
  for _ in 0..NINODE + 1 {
    chdir("..");
    unlink("irefd");
  }

  chdir("/");
}

// test that fork fails gracefully
// the forktest binary also does this, but it runs out of proc entries first.
// inside the bigger usertests binary, we run out of memory first.
pub fn forktest(s: &str) {
  const N: i32 = 1000;

  let mut n = 0;
  while n < N {
    let pid = fork();
    if pid < 0 {
      break;
    }
    if pid == 0 {
      exit(0);
    }
    n += 1;
  }

  if n == 0 {
    printf!("{}: no fork at all!\n", s);
    exit(1);
  }

  if n == N {
    printf!("{}: fork claimed to work 1000 times!\n", s);
    exit(1);
  }

  while n > 0 {
    if wait(None) < 0 {
      printf!("{}: wait stopped early\n", s);
      exit(1);
    }
    n -= 1;
  }

  if wait(None) != -1 {
    printf!("{}: wait got too many\n", s);
    exit(1);
  }
}

pub fn sbrkbasic(s: &str) {
  const TOOMUCH: usize = 1024 * 1024 * 1024;
  let mut xstatus = 0;

  // does sbrk() return the expected failure value?
  let pid = fork();
  if pid < 0 {
    printf!("fork failed in sbrkbasic\n");
    exit(1);
  }
  if pid == 0 {
    let a = sbrk(TOOMUCH as i32);
    if a == SBRK_ERROR {
      // it's OK if this fails.
      exit(0);
    }

    let mut b = a;
    while b < a.wrapping_add(TOOMUCH) {
      unsafe { write_volatile(b, 99) };
      b = b.wrapping_add(PGSIZE);
    }

    // we should not get here! either sbrk(TOOMUCH)
    // should have failed, or (with lazy allocation)
    // a pagefault should have killed this process.
    exit(1);
  }

  wait(Some(&mut xstatus));
  if xstatus == 1 {
    printf!("{}: too much memory allocated!\n", s);
    exit(1);
  }

  // can one sbrk() less than a page?
  let mut a = sbrk(0);
  for i in 0..5000 {
    let b = sbrk(1);
    if b != a {
      printf!("{}: sbrk test failed {} {:p} {:p}\n", s, i, a, b);
      exit(1);
    }
    unsafe { write_volatile(b, 1) };
    a = b.wrapping_add(1);
  }
  let pid = fork();
  if pid < 0 {
    printf!("{}: sbrk test fork failed\n", s);
    exit(1);
  }
  sbrk(1);
  let c = sbrk(1);
  if c != a.wrapping_add(1) {
    printf!("{}: sbrk test failed post-fork\n", s);
    exit(1);
  }
  if pid == 0 {
    exit(0);
  }
  wait(Some(&mut xstatus));
  exit(xstatus);
}

pub fn sbrkmuch(s: &str) {
  const BIG: u64 = 100 * 1024 * 1024;

  let oldbrk = sbrk(0);

  // can one grow address space to something big?
  let a = sbrk(0);
  let amt = BIG - a as u64;
  let p = sbrk(amt as i32);
  if p != a {
    printf!("{}: sbrk test failed to grow big address space; enough phys mem?\n", s);
    exit(1);
  }

  let lastaddr = (BIG - 1) as *mut u8;
  unsafe { write_volatile(lastaddr, 99) };

  // can one de-allocate?
  let a = sbrk(0);
  let c = sbrk(-(PGSIZE as i32));
  if c == SBRK_ERROR {
    printf!("{}: sbrk could not deallocate\n", s);
    exit(1);
  }
  let c = sbrk(0);
  if c != a.wrapping_sub(PGSIZE) {
    printf!("{}: sbrk deallocation produced wrong address, a {:p} c {:p}\n", s, a, c);
    exit(1);
  }

  // can one re-allocate that page?
  let a = sbrk(0);
  let c = sbrk(PGSIZE as i32);
  if c != a || sbrk(0) != a.wrapping_add(PGSIZE) {
    printf!("{}: sbrk re-allocation failed, a {:p} c {:p}\n", s, a, c);
    exit(1);
  }
  if unsafe { read_volatile(lastaddr) } == 99 {
    // should be zero
    printf!("{}: sbrk de-allocation didn't really deallocate\n", s);
    exit(1);
  }

  let a = sbrk(0);
  let c = sbrk(-((sbrk(0) as u64 - oldbrk as u64) as i32));
  if c != a {
    printf!("{}: sbrk downsize failed, a {:p} c {:p}\n", s, a, c);
    exit(1);
  }
}

// can we read the kernel's memory?
pub fn kernmem(s: &str) {
  let mut a = KERNBASE as *const u8;
  while a < (KERNBASE + 2000000) as *const u8 {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid == 0 {
      printf!("{}: oops could read {:p} = {:x}\n", s, a, unsafe { read_volatile(a) });
      exit(1);
    }
    let mut xstatus = 0;
    wait(Some(&mut xstatus));
    if xstatus != -1 {
      // did kernel kill child?
      exit(1);
    }
    a = a.wrapping_add(50000);
  }
}

// user code should not be able to write to addresses above MAXVA.
pub fn MAXVAplus(s: &str) {
  let mut a: u64 = MAXVA;
  while black_box(a) != 0 {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid == 0 {
      unsafe { write_volatile(black_box(a) as *mut u8, 99) };
      printf!("{}: oops wrote {:#x}\n", s, a);
      exit(1);
    }
    let mut xstatus = 0;
    wait(Some(&mut xstatus));
    if xstatus != -1 {
      // did kernel kill child?
      exit(1);
    }
    a <<= 1;
  }
}

// if we run the system out of memory, does it clean up the last
// failed allocation?
pub fn sbrkfail(s: &str) {
  const BIG: u64 = 100 * 1024 * 1024;
  let mut xstatus = 0;
  let mut fds = [0i32; 2];
  let mut scratch = [0u8; 1];
  let mut pids = [0i32; 10];

  let mut failed = false;
  if pipe(&mut fds) != 0 {
    printf!("{}: pipe() failed\n", s);
    exit(1);
  }
  for pid in pids.iter_mut() {
    *pid = fork();
    if *pid == 0 {
      // allocate a lot of memory
      if sbrk((BIG - sbrk(0) as u64) as i32) == SBRK_ERROR {
        write(fds[1], b"0");
      } else {
        write(fds[1], b"1");
      }
      // sit around until killed
      loop {
        pause(1000);
      }
    }
    if *pid != -1 {
      read(fds[0], &mut scratch);
      if scratch[0] == b'0' {
        failed = true;
      }
    }
  }
  if !failed {
    printf!("{}: no allocation failed; allocate more?\n", s);
  }

  // if those failed allocations freed up the pages they did allocate,
  // we'll be able to allocate here
  let c = sbrk(PGSIZE as i32);
  for &pid in pids.iter() {
    if pid == -1 {
      continue;
    }
    kill(pid);
    wait(None);
  }
  if c == SBRK_ERROR {
    printf!("{}: failed sbrk leaked memory\n", s);
    exit(1);
  }

  // test running fork with the above allocated page
  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid == 0 {
    // allocate a lot of memory. this should produce an error
    let a = sbrk((10 * BIG) as i32);
    if a == SBRK_ERROR {
      exit(0);
    }
    printf!("{}: allocate a lot of memory succeeded {}\n", s, 10 * BIG);
    exit(1);
  }
  wait(Some(&mut xstatus));
  if xstatus != 0 {
    exit(1);
  }
}

// test reads/writes from/to allocated memory
pub fn sbrkarg(s: &str) {
  let a = sbrk(PGSIZE as i32);
  let fd = open("sbrk", O_CREATE | O_WRONLY);
  unlink("sbrk");
  if fd < 0 {
    printf!("{}: open sbrk failed\n", s);
    exit(1);
  }
  let n = raw::write(fd, a, PGSIZE as i32);
  if n < 0 {
    printf!("{}: write sbrk failed\n", s);
    exit(1);
  }
  close(fd);

  // test writes to allocated memory
  let a = sbrk(PGSIZE as i32);
  if raw::pipe(a as *mut i32) != 0 {
    printf!("{}: pipe() failed\n", s);
    exit(1);
  }
}

pub fn validatetest(s: &str) {
  let hi: u64 = 1100 * 1024;
  let mut p: u64 = 0;
  while p <= hi {
    // try to crash the kernel by passing in a bad string pointer
    if raw::link(b"nosuchfile\0".as_ptr(), p as *const u8) != -1 {
      printf!("{}: link should not succeed\n", s);
      exit(1);
    }
    p += PGSIZE as u64;
  }
}

// does uninitialized data start out zero?
static mut UNINIT: [u8; 10000] = [0; 10000];

pub fn bsstest(s: &str) {
  for i in 0..10000 {
    if unsafe { read_volatile((&raw const UNINIT as *const u8).add(i)) } != b'\0' {
      printf!("{}: bss test failed\n", s);
      exit(1);
    }
  }
}

// does exec return an error if the arguments
// are larger than a page? or does it write
// below the stack and wreck the instructions/data?
pub fn bigargtest(s: &str) {
  let mut xstatus = 0;

  unlink("bigarg-ok");
  let pid = fork();
  if pid == 0 {
    // the C version uses 400-byte arguments, which are too big
    // for xv6's one-page user stack; scale them with USERSTACK
    // so that they are still too large for this kernel's stack.
    let mut big = [b' '; 400 * USERSTACK];
    let n = big.len();
    big[n - 1] = b'\0';
    let mut args: [*const u8; MAXARG] = [null(); MAXARG];
    for arg in args.iter_mut().take(MAXARG - 1) {
      *arg = big.as_ptr();
    }
    args[MAXARG - 1] = null();
    // this exec() should fail (and return) because the
    // arguments are too large.
    raw::exec(b"echo\0".as_ptr(), args.as_ptr());
    let fd = open("bigarg-ok", O_CREATE);
    close(fd);
    exit(0);
  } else if pid < 0 {
    printf!("{}: bigargtest: fork failed\n", s);
    exit(1);
  }

  wait(Some(&mut xstatus));
  if xstatus != 0 {
    exit(xstatus);
  }
  let fd = open("bigarg-ok", 0);
  if fd < 0 {
    printf!("{}: bigarg test failed!\n", s);
    exit(1);
  }
  close(fd);
}

pub fn argptest(s: &str) {
  let fd = open("init", O_RDONLY);
  if fd < 0 {
    printf!("{}: open failed\n", s);
    exit(1);
  }
  raw::read(fd, sbrk(0).wrapping_sub(1), -1);
  close(fd);
}

// check that there's an invalid page beneath
// the user stack, to catch stack overflow.
pub fn stacktest(s: &str) {
  let mut xstatus = 0;

  let pid = fork();
  if pid == 0 {
    let mut sp = r_sp() as *const u8;
    sp = sp.wrapping_sub(USERSTACK * PGSIZE);
    // the *sp should cause a trap.
    printf!("{}: stacktest: read below stack {}\n", s, unsafe { read_volatile(sp) });
    exit(1);
  } else if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  wait(Some(&mut xstatus));
  if xstatus == -1 {
    // kernel killed child?
    exit(0);
  } else {
    exit(xstatus);
  }
}

// check that writes to a few forbidden addresses
// cause a fault, e.g. process's text and TRAMPOLINE.
pub fn nowrite(s: &str) {
  let mut xstatus = 0;
  let addrs: [u64; 6] = [0, 0x80000000, 0x3fffffe000, 0x3ffffff000, 0x4000000000, 0xffffffffffffffff];

  for &addr in addrs.iter() {
    let pid = fork();
    if pid == 0 {
      let addr = black_box(addr) as *mut i32;
      unsafe { write_volatile(addr, 10) };
      printf!("{}: write to {:p} did not fail!\n", s, addr);
      exit(0);
    } else if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    wait(Some(&mut xstatus));
    if xstatus == 0 {
      // kernel did not kill child!
      exit(1);
    }
  }
  exit(0);
}

// regression test. copyin(), copyout(), and copyinstr() used to cast
// the virtual page address to uint, which (with certain wild system
// call arguments) resulted in a kernel page faults.
const BIG: u64 = 0xeaeb0b5b00002f5e;

pub fn pgbug(_s: &str) {
  let argv: [*const u8; 1] = [null()];
  raw::exec(BIG as *const u8, argv.as_ptr());
  raw::pipe(BIG as *mut i32);

  exit(0);
}

// regression test. does the kernel panic if a process sbrk()s its
// size to be less than a page, or zero, or reduces the break by an
// amount too small to cause a page to be freed?
pub fn sbrkbugs(_s: &str) {
  let pid = fork();
  if pid < 0 {
    printf!("fork failed\n");
    exit(1);
  }
  if pid == 0 {
    let sz = sbrk(0) as u64 as i32;
    // free all user memory; there used to be a bug that
    // would not adjust p->sz correctly in this case,
    // causing exit() to panic.
    sbrk(-sz);
    // user page fault here.
    exit(0);
  }
  wait(None);

  let pid = fork();
  if pid < 0 {
    printf!("fork failed\n");
    exit(1);
  }
  if pid == 0 {
    let sz = sbrk(0) as u64 as i32;
    // set the break to somewhere in the very first
    // page; there used to be a bug that would incorrectly
    // free the first page.
    sbrk(-(sz - 3500));
    exit(0);
  }
  wait(None);

  let pid = fork();
  if pid < 0 {
    printf!("fork failed\n");
    exit(1);
  }
  if pid == 0 {
    // set the break in the middle of a page.
    sbrk(((10 * PGSIZE + 2048) as u64).wrapping_sub(sbrk(0) as u64) as i32);

    // reduce the break a bit, but not enough to
    // cause a page to be freed. this used to cause
    // a panic.
    sbrk(-10);

    exit(0);
  }
  wait(None);

  exit(0);
}

// if process size was somewhat more than a page boundary, and then
// shrunk to be somewhat less than that page boundary, can the kernel
// still copyin() from addresses in the last page?
pub fn sbrklast(_s: &str) {
  let top = sbrk(0) as u64;
  if (top % PGSIZE as u64) != 0 {
    sbrk((PGSIZE as u64 - (top % PGSIZE as u64)) as i32);
  }
  sbrk(PGSIZE as i32);
  sbrk(10);
  sbrk(-20);
  let top = sbrk(0) as u64;
  let p = (top - 64) as *mut u8;
  unsafe {
    write_volatile(p, b'x');
    write_volatile(p.add(1), b'\0');
  }
  let fd = raw::open(p, O_RDWR | O_CREATE);
  raw::write(fd, p, 1);
  close(fd);
  let fd = raw::open(p, O_RDWR);
  unsafe { write_volatile(p, b'\0') };
  raw::read(fd, p, 1);
  if unsafe { read_volatile(p) } != b'x' {
    exit(1);
  }
}

// does sbrk handle signed int32 wrap-around with
// negative arguments?
pub fn sbrk8000(_s: &str) {
  sbrk(0x80000004u32 as i32);
  let top = sbrk(0);
  unsafe {
    let p = top.wrapping_sub(1);
    write_volatile(p, read_volatile(p).wrapping_add(1));
  }
}

// regression test. test whether exec() leaks memory if one of the
// arguments is invalid. the test passes if the kernel doesn't panic.
pub fn badarg(_s: &str) {
  for _ in 0..50000 {
    let argv: [*const u8; 2] = [0xffffffff as *const u8, null()];
    raw::exec(b"echo\0".as_ptr(), argv.as_ptr());
  }

  exit(0);
}

const REGION_SZ: usize = 1024 * 1024 * 1024;

// Touch a page every 64 pages, which with lazy allocation
// causes one page to be allocated.
pub fn lazy_alloc(_s: &str) {
  let prev_end = sbrklazy(REGION_SZ as i32);
  if prev_end == SBRK_ERROR {
    printf!("sbrklazy() failed\n");
    exit(1);
  }
  let new_end = prev_end.wrapping_add(REGION_SZ);

  let mut i = prev_end.wrapping_add(PGSIZE);
  while i < new_end {
    unsafe { write_volatile(i as *mut *mut u8, i) };
    i = i.wrapping_add(64 * PGSIZE);
  }

  let mut i = prev_end.wrapping_add(PGSIZE);
  while i < new_end {
    if unsafe { read_volatile(i as *const *mut u8) } != i {
      printf!("failed to read value from memory\n");
      exit(1);
    }
    i = i.wrapping_add(64 * PGSIZE);
  }

  exit(0);
}

// Touch a page every 64 pages in region, which with lazy allocation
// causes one page to be allocated. Check that freeing the region
// frees the allocated pages.
pub fn lazy_unmap(_s: &str) {
  let prev_end = sbrklazy(REGION_SZ as i32);
  if prev_end == SBRK_ERROR {
    printf!("sbrklazy() failed\n");
    exit(1);
  }
  let new_end = prev_end.wrapping_add(REGION_SZ);

  let mut i = prev_end.wrapping_add(PGSIZE);
  while i < new_end {
    unsafe { write_volatile(i as *mut *mut u8, i) };
    i = i.wrapping_add(PGSIZE * PGSIZE);
  }

  let mut i = prev_end.wrapping_add(PGSIZE);
  while i < new_end {
    let pid = fork();
    if pid < 0 {
      printf!("error forking\n");
      exit(1);
    } else if pid == 0 {
      sbrklazy(-(REGION_SZ as i32));
      unsafe { write_volatile(i as *mut *mut u8, i) };
      exit(0);
    } else {
      let mut status = 0;
      wait(Some(&mut status));
      if status == 0 {
        printf!("memory not unmapped\n");
        exit(1);
      }
    }
    i = i.wrapping_add(PGSIZE * PGSIZE);
  }

  exit(0);
}

pub fn lazy_copy(_s: &str) {
  // copyinstr on lazy page
  {
    let p = sbrk(0);
    sbrklazy(4 * PGSIZE as i32);
    raw::open(p.wrapping_add(8192), 0);
  }

  {
    let xx = sbrk(0);
    let ret = sbrk(-((xx as u64 + 1) as i32));
    if ret != xx {
      printf!("sbrk(sbrk(0)+1) returned {:p}, not old sz\n", ret);
      exit(1);
    }
  }

  // read() and write() to these addresses should fail.
  let bad: [u64; 6] = [0x3fffffc000, 0x3fffffd000, 0x3fffffe000, 0x3ffffff000, 0x4000000000, 0x8000000000];
  for &b in bad.iter() {
    let fd = open("README", 0);
    if fd < 0 {
      printf!("cannot open README\n");
      exit(1);
    }
    if raw::read(fd, b as *mut u8, 512) >= 0 {
      printf!("read succeeded\n");
      exit(1);
    }
    close(fd);
    let fd = open("junk", O_CREATE | O_RDWR | O_TRUNC);
    if fd < 0 {
      printf!("cannot open junk\n");
      exit(1);
    }
    if raw::write(fd, b as *const u8, 512) >= 0 {
      printf!("write succeeded\n");
      exit(1);
    }
    close(fd);
  }

  exit(0);
}
