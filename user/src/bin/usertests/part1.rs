// usertests: see main.rs.
// Ported from xv6's user/usertests.c.

use core::ptr::{null, read_volatile, write_volatile};

use user::*;

use crate::*;

// what if you pass ridiculous pointers to system calls
// that read user memory with copyin?
pub fn copyin(_s: &str) {
  let addrs: [u64; 5] = [0x80000000, 0x3fffffe000, 0x3ffffff000, 0x4000000000, 0xffffffffffffffff];

  for &addr in addrs.iter() {
    let fd = open("copyin1", O_CREATE | O_WRONLY);
    if fd < 0 {
      printf!("open(copyin1) failed\n");
      exit(1);
    }
    let mut n = raw::write(fd, addr as *const u8, 8192);
    if n >= 0 {
      printf!("write(fd, {:#018x}, 8192) returned {}, not -1\n", addr, n);
      exit(1);
    }
    close(fd);
    unlink("copyin1");

    n = raw::write(1, addr as *const u8, 8192);
    if n > 0 {
      printf!("write(1, {:#018x}, 8192) returned {}, not -1 or 0\n", addr, n);
      exit(1);
    }

    let mut fds = [0i32; 2];
    if pipe(&mut fds) < 0 {
      printf!("pipe() failed\n");
      exit(1);
    }
    n = raw::write(fds[1], addr as *const u8, 8192);
    if n > 0 {
      printf!("write(pipe, {:#018x}, 8192) returned {}, not -1 or 0\n", addr, n);
      exit(1);
    }
    close(fds[0]);
    close(fds[1]);
  }
}

// what if you pass ridiculous pointers to system calls
// that write user memory with copyout?
pub fn copyout(_s: &str) {
  let addrs: [u64; 6] = [0, 0x80000000, 0x3fffffe000, 0x3ffffff000, 0x4000000000, 0xffffffffffffffff];

  for &addr in addrs.iter() {
    let fd = open("README", 0);
    if fd < 0 {
      printf!("open(README) failed\n");
      exit(1);
    }
    let mut n = raw::read(fd, addr as *mut u8, 8192);
    if n > 0 {
      printf!("read(fd, {:#018x}, 8192) returned {}, not -1 or 0\n", addr, n);
      exit(1);
    }
    close(fd);

    let mut fds = [0i32; 2];
    if pipe(&mut fds) < 0 {
      printf!("pipe() failed\n");
      exit(1);
    }
    n = write(fds[1], b"x");
    if n != 1 {
      printf!("pipe write failed\n");
      exit(1);
    }
    n = raw::read(fds[0], addr as *mut u8, 8192);
    if n > 0 {
      printf!("read(pipe, {:#018x}, 8192) returned {}, not -1 or 0\n", addr, n);
      exit(1);
    }
    close(fds[0]);
    close(fds[1]);
  }
}

// what if you pass ridiculous string pointers to system calls?
pub fn copyinstr1(_s: &str) {
  let addrs: [u64; 5] = [0x80000000, 0x3fffffe000, 0x3ffffff000, 0x4000000000, 0xffffffffffffffff];

  for &addr in addrs.iter() {
    let fd = raw::open(addr as *const u8, O_CREATE | O_WRONLY);
    if fd >= 0 {
      printf!("open({:#018x}) returned {}, not -1\n", addr, fd);
      exit(1);
    }
  }
}

// what if a string system call argument is exactly the size
// of the kernel buffer it is copied into, so that the null
// would fall just beyond the end of the kernel buffer?
pub fn copyinstr2(_s: &str) {
  let mut b = [0u8; MAXPATH + 1];

  for c in b.iter_mut().take(MAXPATH) {
    *c = b'x';
  }
  b[MAXPATH] = 0;

  let mut ret = unlink(&b);
  if ret != -1 {
    printf!("unlink({}) returned {}, not -1\n", as_str(&b), ret);
    exit(1);
  }

  let fd = open(&b, O_CREATE | O_WRONLY);
  if fd != -1 {
    printf!("open({}) returned {}, not -1\n", as_str(&b), fd);
    exit(1);
  }

  ret = link(&b, &b);
  if ret != -1 {
    printf!("link({}, {}) returned {}, not -1\n", as_str(&b), as_str(&b), ret);
    exit(1);
  }

  let args = ["xx"];
  ret = exec(&b, &args);
  if ret != -1 {
    printf!("exec({}) returned {}, not -1\n", as_str(&b), fd);
    exit(1);
  }

  let pid = fork();
  if pid < 0 {
    printf!("fork failed\n");
    exit(1);
  }
  if pid == 0 {
    static mut BIG: [u8; PGSIZE + 1] = [0; PGSIZE + 1];
    let big = unsafe { &mut *(&raw mut BIG) };
    for c in big.iter_mut().take(PGSIZE) {
      *c = b'x';
    }
    big[PGSIZE] = 0;
    let args2: [&[u8]; 3] = [&big[..], &big[..], &big[..]];
    ret = exec("echo", &args2);
    if ret != -1 {
      printf!("exec(echo, BIG) returned {}, not -1\n", fd);
      exit(1);
    }
    exit(747); // OK
  }

  let mut st = 0;
  wait(Some(&mut st));
  if st != 747 {
    printf!("exec(echo, BIG) succeeded, should have failed\n");
    exit(1);
  }
}

// what if a string argument crosses over the end of last user page?
pub fn copyinstr3(_s: &str) {
  sbrk(8192);
  let mut top = sbrk(0) as u64;
  if (top % PGSIZE as u64) != 0 {
    sbrk((PGSIZE as u64 - (top % PGSIZE as u64)) as i32);
  }
  top = sbrk(0) as u64;
  if top % PGSIZE as u64 != 0 {
    printf!("oops\n");
    exit(1);
  }

  let b = (top - 1) as *mut u8;
  unsafe { write_volatile(b, b'x') };

  let mut ret = raw::unlink(b);
  if ret != -1 {
    printf!("unlink({}) returned {}, not -1\n", "x", ret);
    exit(1);
  }

  let fd = raw::open(b, O_CREATE | O_WRONLY);
  if fd != -1 {
    printf!("open({}) returned {}, not -1\n", "x", fd);
    exit(1);
  }

  ret = raw::link(b, b);
  if ret != -1 {
    printf!("link({}, {}) returned {}, not -1\n", "x", "x", ret);
    exit(1);
  }

  let args: [*const u8; 2] = [b"xx\0".as_ptr(), null()];
  ret = raw::exec(b, args.as_ptr());
  if ret != -1 {
    printf!("exec({}) returned {}, not -1\n", "x", fd);
    exit(1);
  }
}

// See if the kernel refuses to read/write user memory that the
// application doesn't have anymore, because it returned it.
pub fn rwsbrk(_s: &str) {
  let a = sbrk(8192) as u64;

  if a == SBRK_ERROR as u64 {
    printf!("sbrk(rwsbrk) failed\n");
    exit(1);
  }

  if sbrk(-8192) == SBRK_ERROR {
    printf!("sbrk(rwsbrk) shrink failed\n");
    exit(1);
  }

  let mut fd = open("rwsbrk", O_CREATE | O_WRONLY);
  if fd < 0 {
    printf!("open(rwsbrk) failed\n");
    exit(1);
  }
  let mut n = raw::write(fd, (a + PGSIZE as u64) as *const u8, 1024);
  if n >= 0 {
    printf!("write(fd, {:#018x}, 1024) returned {}, not -1\n", a + PGSIZE as u64, n);
    exit(1);
  }
  close(fd);
  unlink("rwsbrk");

  fd = open("README", O_RDONLY);
  if fd < 0 {
    printf!("open(README) failed\n");
    exit(1);
  }
  n = raw::read(fd, (a + PGSIZE as u64) as *mut u8, 10);
  if n >= 0 {
    printf!("read(fd, {:#018x}, 10) returned {}, not -1\n", a + PGSIZE as u64, n);
    exit(1);
  }
  close(fd);

  exit(0);
}

// test O_TRUNC.
pub fn truncate1(s: &str) {
  let mut buf = [0u8; 32];

  unlink("truncfile");
  let mut fd1 = open("truncfile", O_CREATE | O_WRONLY | O_TRUNC);
  write(fd1, b"abcd");
  close(fd1);

  let fd2 = open("truncfile", O_RDONLY);
  let mut n = read(fd2, &mut buf);
  if n != 4 {
    printf!("{}: read {} bytes, wanted 4\n", s, n);
    exit(1);
  }

  fd1 = open("truncfile", O_WRONLY | O_TRUNC);

  let fd3 = open("truncfile", O_RDONLY);
  n = read(fd3, &mut buf);
  if n != 0 {
    printf!("aaa fd3={}\n", fd3);
    printf!("{}: read {} bytes, wanted 0\n", s, n);
    exit(1);
  }

  n = read(fd2, &mut buf);
  if n != 0 {
    printf!("bbb fd2={}\n", fd2);
    printf!("{}: read {} bytes, wanted 0\n", s, n);
    exit(1);
  }

  write(fd1, b"abcdef");

  n = read(fd3, &mut buf);
  if n != 6 {
    printf!("{}: read {} bytes, wanted 6\n", s, n);
    exit(1);
  }

  n = read(fd2, &mut buf);
  if n != 2 {
    printf!("{}: read {} bytes, wanted 2\n", s, n);
    exit(1);
  }

  unlink("truncfile");

  close(fd1);
  close(fd2);
  close(fd3);
}

// write to an open FD whose file has just been truncated.
// this causes a write at an offset beyond the end of the file.
// such writes fail on xv6 (unlike POSIX) but at least
// they don't crash.
pub fn truncate2(s: &str) {
  unlink("truncfile");

  let fd1 = open("truncfile", O_CREATE | O_TRUNC | O_WRONLY);
  write(fd1, b"abcd");

  let fd2 = open("truncfile", O_TRUNC | O_WRONLY);

  let n = write(fd1, b"x");
  if n != -1 {
    printf!("{}: write returned {}, expected -1\n", s, n);
    exit(1);
  }

  unlink("truncfile");
  close(fd1);
  close(fd2);
}

pub fn truncate3(s: &str) {
  let mut xstatus = 0;

  close(open("truncfile", O_CREATE | O_TRUNC | O_WRONLY));

  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }

  if pid == 0 {
    for _ in 0..100 {
      let mut buf = [0u8; 32];
      let mut fd = open("truncfile", O_WRONLY);
      if fd < 0 {
        printf!("{}: open failed\n", s);
        exit(1);
      }
      let n = write(fd, b"1234567890");
      if n != 10 {
        printf!("{}: write got {}, expected 10\n", s, n);
        exit(1);
      }
      close(fd);
      fd = open("truncfile", O_RDONLY);
      read(fd, &mut buf);
      close(fd);
    }
    exit(0);
  }

  for _ in 0..150 {
    let fd = open("truncfile", O_CREATE | O_WRONLY | O_TRUNC);
    if fd < 0 {
      printf!("{}: open failed\n", s);
      exit(1);
    }
    let n = write(fd, b"xxx");
    if n != 3 {
      printf!("{}: write got {}, expected 3\n", s, n);
      exit(1);
    }
    close(fd);
  }

  wait(Some(&mut xstatus));
  unlink("truncfile");
  exit(xstatus);
}

// does chdir() call iput(p->cwd) in a transaction?
pub fn iputtest(s: &str) {
  if mkdir("iputdir") < 0 {
    printf!("{}: mkdir failed\n", s);
    exit(1);
  }
  if chdir("iputdir") < 0 {
    printf!("{}: chdir iputdir failed\n", s);
    exit(1);
  }
  if unlink("../iputdir") < 0 {
    printf!("{}: unlink ../iputdir failed\n", s);
    exit(1);
  }
  if chdir("/") < 0 {
    printf!("{}: chdir / failed\n", s);
    exit(1);
  }
}

// does exit() call iput(p->cwd) in a transaction?
pub fn exitiputtest(s: &str) {
  let mut xstatus = 0;

  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid == 0 {
    if mkdir("iputdir") < 0 {
      printf!("{}: mkdir failed\n", s);
      exit(1);
    }
    if chdir("iputdir") < 0 {
      printf!("{}: child chdir failed\n", s);
      exit(1);
    }
    if unlink("../iputdir") < 0 {
      printf!("{}: unlink ../iputdir failed\n", s);
      exit(1);
    }
    exit(0);
  }
  wait(Some(&mut xstatus));
  exit(xstatus);
}

// does the error path in open() for attempt to write a
// directory call iput() in a transaction?
// needs a hacked kernel that pauses just after the namei()
// call in sys_open():
//    if((ip = namei(path)) == 0)
//      return -1;
//    {
//      int i;
//      for(i = 0; i < 10000; i++)
//        yield();
//    }
pub fn openiputtest(s: &str) {
  let mut xstatus = 0;

  if mkdir("oidir") < 0 {
    printf!("{}: mkdir oidir failed\n", s);
    exit(1);
  }
  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid == 0 {
    let fd = open("oidir", O_RDWR);
    if fd >= 0 {
      printf!("{}: open directory for write succeeded\n", s);
      exit(1);
    }
    exit(0);
  }
  pause(1);
  if unlink("oidir") != 0 {
    printf!("{}: unlink failed\n", s);
    exit(1);
  }
  wait(Some(&mut xstatus));
  exit(xstatus);
}

// simple file system tests

pub fn opentest(s: &str) {
  let mut fd = open("echo", 0);
  if fd < 0 {
    printf!("{}: open echo failed!\n", s);
    exit(1);
  }
  close(fd);
  fd = open("doesnotexist", 0);
  if fd >= 0 {
    printf!("{}: open doesnotexist succeeded!\n", s);
    exit(1);
  }
}

pub fn writetest(s: &str) {
  const N: usize = 100;
  const SZ: usize = 10;

  let mut fd = open("small", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: error: creat small failed!\n", s);
    exit(1);
  }
  for i in 0..N {
    if write(fd, &b"aaaaaaaaaa"[..SZ]) != SZ as i32 {
      printf!("{}: error: write aa {} new file failed\n", s, i);
      exit(1);
    }
    if write(fd, &b"bbbbbbbbbb"[..SZ]) != SZ as i32 {
      printf!("{}: error: write bb {} new file failed\n", s, i);
      exit(1);
    }
  }
  close(fd);
  fd = open("small", O_RDONLY);
  if fd < 0 {
    printf!("{}: error: open small failed!\n", s);
    exit(1);
  }
  let i = read(fd, &mut buf()[..N * SZ * 2]);
  if i != (N * SZ * 2) as i32 {
    printf!("{}: read failed\n", s);
    exit(1);
  }
  close(fd);

  if unlink("small") < 0 {
    printf!("{}: unlink small failed\n", s);
    exit(1);
  }
}

pub fn writebig(s: &str) {
  let buf = buf();

  let mut fd = open("big", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: error: creat big failed!\n", s);
    exit(1);
  }

  for i in 0..MAXFILE {
    buf[..4].copy_from_slice(&(i as i32).to_ne_bytes());
    if write(fd, &buf[..BSIZE]) != BSIZE as i32 {
      printf!("{}: error: write big file failed i={}\n", s, i);
      exit(1);
    }
  }

  close(fd);

  fd = open("big", O_RDONLY);
  if fd < 0 {
    printf!("{}: error: open big failed!\n", s);
    exit(1);
  }

  let mut n: i32 = 0;
  loop {
    let i = read(fd, &mut buf[..BSIZE]);
    if i == 0 {
      if n != MAXFILE as i32 {
        printf!("{}: read only {} blocks from big", s, n);
        exit(1);
      }
      break;
    } else if i != BSIZE as i32 {
      printf!("{}: read failed {}\n", s, i);
      exit(1);
    }
    let v = i32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if v != n {
      printf!("{}: read content of block {} is {}\n", s, n, v);
      exit(1);
    }
    n += 1;
  }
  close(fd);
  if unlink("big") < 0 {
    printf!("{}: unlink big failed\n", s);
    exit(1);
  }
}

// many creates, followed by unlink test
pub fn createtest(_s: &str) {
  const N: u8 = 52;

  let mut name = [0u8; 3];
  name[0] = b'a';
  name[2] = 0;
  for i in 0..N {
    name[1] = b'0' + i;
    let fd = open(&name, O_CREATE | O_RDWR);
    close(fd);
  }
  name[0] = b'a';
  name[2] = 0;
  for i in 0..N {
    name[1] = b'0' + i;
    unlink(&name);
  }
}

pub fn dirtest(s: &str) {
  if mkdir("dir0") < 0 {
    printf!("{}: mkdir failed\n", s);
    exit(1);
  }

  if chdir("dir0") < 0 {
    printf!("{}: chdir dir0 failed\n", s);
    exit(1);
  }

  if chdir("..") < 0 {
    printf!("{}: chdir .. failed\n", s);
    exit(1);
  }

  if unlink("dir0") < 0 {
    printf!("{}: unlink dir0 failed\n", s);
    exit(1);
  }
}

pub fn exectest(s: &str) {
  let mut xstatus = 0;
  let echoargv = ["echo", "OK"];
  let mut buf = [0u8; 3];

  unlink("echo-ok");
  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid == 0 {
    close(1);
    let fd = open("echo-ok", O_CREATE | O_WRONLY);
    if fd < 0 {
      printf!("{}: create failed\n", s);
      exit(1);
    }
    if fd != 1 {
      printf!("{}: wrong fd\n", s);
      exit(1);
    }
    if exec("echo", &echoargv) < 0 {
      printf!("{}: exec echo failed\n", s);
      exit(1);
    }
    // won't get to here
  }
  if wait(Some(&mut xstatus)) != pid {
    printf!("{}: wait failed!\n", s);
  }
  if xstatus != 0 {
    exit(xstatus);
  }

  let fd = open("echo-ok", O_RDONLY);
  if fd < 0 {
    printf!("{}: open failed\n", s);
    exit(1);
  }
  if read(fd, &mut buf[..2]) != 2 {
    printf!("{}: read failed\n", s);
    exit(1);
  }
  unlink("echo-ok");
  if buf[0] == b'O' && buf[1] == b'K' {
    exit(0);
  } else {
    printf!("{}: wrong output\n", s);
    exit(1);
  }
}

// simple fork and pipe read/write

pub fn pipe1(s: &str) {
  const N: usize = 5;
  const SZ: usize = 1033;
  let buf = buf();
  let mut fds = [0i32; 2];
  let mut xstatus = 0;

  if pipe(&mut fds) != 0 {
    printf!("{}: pipe() failed\n", s);
    exit(1);
  }
  let pid = fork();
  let mut seq: i32 = 0;
  if pid == 0 {
    close(fds[0]);
    for _ in 0..N {
      for b in buf.iter_mut().take(SZ) {
        *b = seq as u8;
        seq += 1;
      }
      if write(fds[1], &buf[..SZ]) != SZ as i32 {
        printf!("{}: pipe1 oops 1\n", s);
        exit(1);
      }
    }
    exit(0);
  } else if pid > 0 {
    close(fds[1]);
    let mut total = 0;
    let mut cc = 1;
    loop {
      let n = read(fds[0], &mut buf[..cc]);
      if n <= 0 {
        break;
      }
      for &b in buf.iter().take(n as usize) {
        if (b as i32 & 0xff) != (seq & 0xff) {
          printf!("{}: pipe1 oops 2\n", s);
          return;
        }
        seq += 1;
      }
      total += n as usize;
      cc *= 2;
      if cc > BUFSZ {
        cc = BUFSZ;
      }
    }
    if total != N * SZ {
      printf!("{}: pipe1 oops 3 total {}\n", s, total);
      exit(1);
    }
    close(fds[0]);
    wait(Some(&mut xstatus));
    exit(xstatus);
  } else {
    printf!("{}: fork() failed\n", s);
    exit(1);
  }
}

// test if child is killed (status = -1)
pub fn killstatus(s: &str) {
  let mut xst = 0;

  for _ in 0..100 {
    let pid1 = fork();
    if pid1 < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid1 == 0 {
      loop {
        getpid();
      }
    }
    pause(1);
    kill(pid1);
    wait(Some(&mut xst));
    if xst != -1 {
      printf!("{}: status should be -1\n", s);
      exit(1);
    }
  }
  exit(0);
}

// meant to be run w/ at most two CPUs
pub fn preempt(s: &str) {
  let mut pfds = [0i32; 2];

  let pid1 = fork();
  if pid1 < 0 {
    printf!("{}: fork failed", s);
    exit(1);
  }
  if pid1 == 0 {
    #[allow(clippy::empty_loop)]
    loop {}
  }

  let pid2 = fork();
  if pid2 < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid2 == 0 {
    #[allow(clippy::empty_loop)]
    loop {}
  }

  pipe(&mut pfds);
  let pid3 = fork();
  if pid3 < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }
  if pid3 == 0 {
    close(pfds[0]);
    if write(pfds[1], b"x") != 1 {
      printf!("{}: preempt write error", s);
    }
    close(pfds[1]);
    #[allow(clippy::empty_loop)]
    loop {}
  }

  close(pfds[1]);
  if read(pfds[0], buf()) != 1 {
    printf!("{}: preempt read error", s);
    return;
  }
  close(pfds[0]);
  printf!("kill... ");
  kill(pid1);
  kill(pid2);
  kill(pid3);
  printf!("wait... ");
  wait(None);
  wait(None);
  wait(None);
}

// try to find any races between exit and wait
pub fn exitwait(s: &str) {
  for i in 0..100 {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid != 0 {
      let mut xstate = 0;
      if wait(Some(&mut xstate)) != pid {
        printf!("{}: wait wrong pid\n", s);
        exit(1);
      }
      if i != xstate {
        printf!("{}: wait wrong exit status\n", s);
        exit(1);
      }
    } else {
      exit(i);
    }
  }
}

// try to find races in the reparenting
// code that handles a parent exiting
// when it still has live children.
pub fn reparent(s: &str) {
  let master_pid = getpid();
  for _ in 0..200 {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid != 0 {
      if wait(None) != pid {
        printf!("{}: wait wrong pid\n", s);
        exit(1);
      }
    } else {
      let pid2 = fork();
      if pid2 < 0 {
        kill(master_pid);
        exit(1);
      }
      exit(0);
    }
  }
  exit(0);
}

// what if two children exit() at the same time?
pub fn twochildren(s: &str) {
  for _ in 0..1000 {
    let pid1 = fork();
    if pid1 < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if pid1 == 0 {
      exit(0);
    } else {
      let pid2 = fork();
      if pid2 < 0 {
        printf!("{}: fork failed\n", s);
        exit(1);
      }
      if pid2 == 0 {
        exit(0);
      } else {
        wait(None);
        wait(None);
      }
    }
  }
}

// concurrent forks to try to expose locking bugs.
pub fn forkfork(s: &str) {
  const N: usize = 2;

  for _ in 0..N {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed", s);
      exit(1);
    }
    if pid == 0 {
      for _ in 0..200 {
        let pid1 = fork();
        if pid1 < 0 {
          exit(1);
        }
        if pid1 == 0 {
          exit(0);
        }
        wait(None);
      }
      exit(0);
    }
  }

  let mut xstatus = 0;
  for _ in 0..N {
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      printf!("{}: fork in child failed", s);
      exit(1);
    }
  }
}

pub fn forkforkfork(s: &str) {
  unlink("stopforking");

  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed", s);
    exit(1);
  }
  if pid == 0 {
    loop {
      let fd = open("stopforking", 0);
      if fd >= 0 {
        exit(0);
      }
      if fork() < 0 {
        close(open("stopforking", O_CREATE | O_RDWR));
      }
    }
  }

  pause(20); // two seconds
  close(open("stopforking", O_CREATE | O_RDWR));
  wait(None);
  pause(10); // one second
}

// regression test. does reparent() violate the parent-then-child
// locking order when giving away a child to init, so that exit()
// deadlocks against init's wait()? also used to trigger a "panic:
// release" due to exit() releasing a different p->parent->lock than
// it acquired.
pub fn reparent2(_s: &str) {
  for _ in 0..800 {
    let pid1 = fork();
    if pid1 < 0 {
      printf!("fork failed\n");
      exit(1);
    }
    if pid1 == 0 {
      fork();
      fork();
      exit(0);
    }
    wait(None);
  }

  exit(0);
}

// allocate all mem, free it, and allocate again
pub fn mem(s: &str) {
  let pid = fork();
  if pid == 0 {
    let mut m1: *mut u8 = core::ptr::null_mut();
    loop {
      let m2 = malloc(10001);
      if m2.is_null() {
        break;
      }
      unsafe { write_volatile(m2 as *mut *mut u8, m1) };
      m1 = m2;
    }
    while !m1.is_null() {
      let m2 = unsafe { read_volatile(m1 as *mut *mut u8) };
      free(m1);
      m1 = m2;
    }
    m1 = malloc(1024 * 20);
    if m1.is_null() {
      printf!("{}: couldn't allocate mem?!!\n", s);
      exit(1);
    }
    free(m1);
    exit(0);
  } else {
    let mut xstatus = 0;
    wait(Some(&mut xstatus));
    if xstatus == -1 {
      // probably page fault, so might be lazy lab,
      // so OK.
      exit(0);
    }
    exit(xstatus);
  }
}

// More file system tests

// two processes write to the same file descriptor
// is the offset shared? does inode locking work?
pub fn sharedfd(s: &str) {
  const N: usize = 1000;
  const SZ: usize = 10;
  let mut buf = [0u8; SZ];

  unlink("sharedfd");
  let mut fd = open("sharedfd", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: cannot open sharedfd for writing", s);
    exit(1);
  }
  let pid = fork();
  buf.fill(if pid == 0 { b'c' } else { b'p' });
  for _ in 0..N {
    if write(fd, &buf) != SZ as i32 {
      printf!("{}: write sharedfd failed\n", s);
      exit(1);
    }
  }
  if pid == 0 {
    exit(0);
  } else {
    let mut xstatus = 0;
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      exit(xstatus);
    }
  }

  close(fd);
  fd = open("sharedfd", 0);
  if fd < 0 {
    printf!("{}: cannot open sharedfd for reading\n", s);
    exit(1);
  }
  let mut nc = 0;
  let mut np = 0;
  while read(fd, &mut buf) > 0 {
    for &c in buf.iter() {
      if c == b'c' {
        nc += 1;
      }
      if c == b'p' {
        np += 1;
      }
    }
  }
  close(fd);
  unlink("sharedfd");
  if nc == N * SZ && np == N * SZ {
    exit(0);
  } else {
    printf!("{}: nc/np test fails\n", s);
    exit(1);
  }
}

// four processes write different files at the same
// time, to test block allocation.
pub fn fourfiles(s: &str) {
  let names = ["f0", "f1", "f2", "f3"];
  const N: usize = 12;
  const NCHILD: usize = 4;
  const SZ: usize = 500;
  let buf = buf();

  for (pi, &fname) in names.iter().enumerate().take(NCHILD) {
    unlink(fname);

    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }

    if pid == 0 {
      let fd = open(fname, O_CREATE | O_RDWR);
      if fd < 0 {
        printf!("{}: create failed\n", s);
        exit(1);
      }

      buf[..SZ].fill(b'0' + pi as u8);
      for _ in 0..N {
        let n = write(fd, &buf[..SZ]);
        if n != SZ as i32 {
          printf!("write failed {}\n", n);
          exit(1);
        }
      }
      exit(0);
    }
  }

  let mut xstatus = 0;
  for _ in 0..NCHILD {
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      exit(xstatus);
    }
  }

  for (i, &fname) in names.iter().enumerate().take(NCHILD) {
    let fd = open(fname, 0);
    let mut total = 0;
    loop {
      let n = read(fd, &mut buf[..]);
      if n <= 0 {
        break;
      }
      for &c in buf.iter().take(n as usize) {
        if c != b'0' + i as u8 {
          printf!("{}: wrong char\n", s);
          exit(1);
        }
      }
      total += n as usize;
    }
    close(fd);
    if total != N * SZ {
      printf!("wrong length {}\n", total);
      exit(1);
    }
    unlink(fname);
  }
}

// four processes create and delete different files in same directory
pub fn createdelete(s: &str) {
  const N: u8 = 20;
  const NCHILD: u8 = 4;
  let mut name = [0u8; 32];

  for pi in 0..NCHILD {
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }

    if pid == 0 {
      name[0] = b'p' + pi;
      name[2] = 0;
      for i in 0..N {
        name[1] = b'0' + i;
        let fd = open(&name, O_CREATE | O_RDWR);
        if fd < 0 {
          printf!("{}: create failed\n", s);
          exit(1);
        }
        close(fd);
        if i > 0 && (i % 2) == 0 {
          name[1] = b'0' + (i / 2);
          if unlink(&name) < 0 {
            printf!("{}: unlink failed\n", s);
            exit(1);
          }
        }
      }
      exit(0);
    }
  }

  let mut xstatus = 0;
  for _ in 0..NCHILD {
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      exit(1);
    }
  }

  name[0] = 0;
  name[1] = 0;
  name[2] = 0;
  for i in 0..N {
    for pi in 0..NCHILD {
      name[0] = b'p' + pi;
      name[1] = b'0' + i;
      let fd = open(&name, 0);
      if (i == 0 || i >= N / 2) && fd < 0 {
        printf!("{}: oops createdelete {} didn't exist\n", s, as_str(&name));
        exit(1);
      } else if (1..N / 2).contains(&i) && fd >= 0 {
        printf!("{}: oops createdelete {} did exist\n", s, as_str(&name));
        exit(1);
      }
      if fd >= 0 {
        close(fd);
      }
    }
  }

  for i in 0..N {
    for pi in 0..NCHILD {
      name[0] = b'p' + pi;
      name[1] = b'0' + i;
      unlink(&name);
    }
  }
}

// can I unlink a file and still read it?
pub fn unlinkread(s: &str) {
  const SZ: i32 = 5;

  let mut fd = open("unlinkread", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: create unlinkread failed\n", s);
    exit(1);
  }
  write(fd, b"hello");
  close(fd);

  fd = open("unlinkread", O_RDWR);
  if fd < 0 {
    printf!("{}: open unlinkread failed\n", s);
    exit(1);
  }
  if unlink("unlinkread") != 0 {
    printf!("{}: unlink unlinkread failed\n", s);
    exit(1);
  }

  let fd1 = open("unlinkread", O_CREATE | O_RDWR);
  write(fd1, b"yyy");
  close(fd1);

  let buf = buf();
  if read(fd, &mut buf[..]) != SZ {
    printf!("{}: unlinkread read failed", s);
    exit(1);
  }
  if buf[0] != b'h' {
    printf!("{}: unlinkread wrong data\n", s);
    exit(1);
  }
  if write(fd, &buf[..10]) != 10 {
    printf!("{}: unlinkread write failed\n", s);
    exit(1);
  }
  close(fd);
  unlink("unlinkread");
}

pub fn linktest(s: &str) {
  const SZ: i32 = 5;

  unlink("lf1");
  unlink("lf2");

  let mut fd = open("lf1", O_CREATE | O_RDWR);
  if fd < 0 {
    printf!("{}: create lf1 failed\n", s);
    exit(1);
  }
  if write(fd, b"hello") != SZ {
    printf!("{}: write lf1 failed\n", s);
    exit(1);
  }
  close(fd);

  if link("lf1", "lf2") < 0 {
    printf!("{}: link lf1 lf2 failed\n", s);
    exit(1);
  }
  unlink("lf1");

  if open("lf1", 0) >= 0 {
    printf!("{}: unlinked lf1 but it is still there!\n", s);
    exit(1);
  }

  fd = open("lf2", 0);
  if fd < 0 {
    printf!("{}: open lf2 failed\n", s);
    exit(1);
  }
  if read(fd, &mut buf()[..]) != SZ {
    printf!("{}: read lf2 failed\n", s);
    exit(1);
  }
  close(fd);

  if link("lf2", "lf2") >= 0 {
    printf!("{}: link lf2 lf2 succeeded! oops\n", s);
    exit(1);
  }

  unlink("lf2");
  if link("lf2", "lf1") >= 0 {
    printf!("{}: link non-existent succeeded! oops\n", s);
    exit(1);
  }

  if link(".", "lf1") >= 0 {
    printf!("{}: link . lf1 succeeded! oops\n", s);
    exit(1);
  }
}

// test concurrent create/link/unlink of the same file
pub fn concreate(s: &str) {
  const N: usize = 40;
  let mut file = [0u8; 3];
  let mut fa = [0u8; N];
  let mut de = Dirent::default();

  file[0] = b'C';
  file[2] = 0;
  for i in 0..N {
    file[1] = b'0' + i as u8;
    unlink(&file);
    let pid = fork();
    if pid != 0 && (i % 3) == 1 {
      link("C0", &file);
    } else if pid == 0 && (i % 5) == 1 {
      link("C0", &file);
    } else {
      let fd = open(&file, O_CREATE | O_RDWR);
      if fd < 0 {
        printf!("concreate create {} failed\n", as_str(&file));
        exit(1);
      }
      close(fd);
    }
    if pid == 0 {
      exit(0);
    } else {
      let mut xstatus = 0;
      wait(Some(&mut xstatus));
      if xstatus != 0 {
        exit(1);
      }
    }
  }

  fa.fill(0);
  let fd = open(".", 0);
  let mut n = 0;
  while raw::read(fd, &raw mut de as *mut u8, size_of::<Dirent>() as i32) > 0 {
    if de.inum == 0 {
      continue;
    }
    if de.name[0] == b'C' && de.name[2] == 0 {
      let i = de.name[1] as i32 - b'0' as i32;
      if i < 0 || i >= fa.len() as i32 {
        printf!("{}: concreate weird file {}\n", s, as_str(&de.name));
        exit(1);
      }
      if fa[i as usize] != 0 {
        printf!("{}: concreate duplicate file {}\n", s, as_str(&de.name));
        exit(1);
      }
      fa[i as usize] = 1;
      n += 1;
    }
  }
  close(fd);

  if n != N {
    printf!("{}: concreate not enough files in directory listing\n", s);
    exit(1);
  }

  for i in 0..N {
    file[1] = b'0' + i as u8;
    let pid = fork();
    if pid < 0 {
      printf!("{}: fork failed\n", s);
      exit(1);
    }
    if ((i % 3) == 0 && pid == 0) || ((i % 3) == 1 && pid != 0) {
      close(open(&file, 0));
      close(open(&file, 0));
      close(open(&file, 0));
      close(open(&file, 0));
      close(open(&file, 0));
      close(open(&file, 0));
    } else {
      unlink(&file);
      unlink(&file);
      unlink(&file);
      unlink(&file);
      unlink(&file);
      unlink(&file);
    }
    if pid == 0 {
      exit(0);
    } else {
      wait(None);
    }
  }
}

// another concurrent link/unlink/create test,
// to look for deadlocks.
pub fn linkunlink(s: &str) {
  unlink("x");
  let pid = fork();
  if pid < 0 {
    printf!("{}: fork failed\n", s);
    exit(1);
  }

  let mut x: u32 = if pid != 0 { 1 } else { 97 };
  for _ in 0..100 {
    x = x.wrapping_mul(1103515245).wrapping_add(12345);
    if (x % 3) == 0 {
      close(open("x", O_RDWR | O_CREATE));
    } else if (x % 3) == 1 {
      link("cat", "x");
    } else {
      unlink("x");
    }
  }

  if pid != 0 {
    wait(None);
  } else {
    exit(0);
  }
}
