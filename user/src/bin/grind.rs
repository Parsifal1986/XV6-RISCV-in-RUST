//
// run random system calls in parallel forever.
//

#![no_std]
#![no_main]

use core::ffi::CStr;
use core::ptr;

use user::*;

// from FreeBSD.
fn do_rand(ctx: &mut u64) -> i32 {
  //
  // Compute x = (7^5 * x) mod (2^31 - 1)
  // without overflowing 31 bits:
  //      (2^31 - 1) = 127773 * (7^5) + 2836
  // From "Random number generators: good ones are hard to find",
  // Park and Miller, Communications of the ACM, vol. 31, no. 10,
  // October 1988, p. 1195.
  //

  // Transform to [1, 0x7ffffffe] range.
  let mut x: i64 = (*ctx % 0x7ffffffe) as i64 + 1;
  let hi = x / 127773;
  let lo = x % 127773;
  x = 16807 * lo - 2836 * hi;
  if x < 0 {
    x += 0x7fffffff;
  }
  // Transform to [0, 0x7ffffffd] range.
  x -= 1;
  *ctx = x as u64;
  x as i32
}

static mut RAND_NEXT: u64 = 1;

fn rand() -> i32 {
  let ctx = &raw mut RAND_NEXT;
  unsafe { do_rand(&mut *ctx) }
}

static mut BUF: [u8; 999] = [0; 999];

// exec() without malloc() (go() shrinks the heap back to its
// initial break, which would pull memory out from under malloc).
// argv is a null-terminated list of NUL-terminated strings.
fn exec0(path: &CStr, argv: &[*const u8]) -> i32 {
  raw::exec(path.as_ptr().cast(), argv.as_ptr())
}

fn go(which_child: i32) -> ! {
  let mut fd = -1;
  let buf = &raw mut BUF;
  let buf = unsafe { &mut *buf };
  let break0 = sbrk(0);
  let mut iters: u64 = 0;

  mkdir("grindir");
  if chdir("grindir") != 0 {
    printf!("grind: chdir grindir failed\n");
    exit(1);
  }
  chdir("/");

  loop {
    iters += 1;
    if iters.is_multiple_of(500) {
      write(1, if which_child != 0 { b"B" } else { b"A" });
    }
    let what = rand() % 23;
    if what == 1 {
      close(open("grindir/../a", O_CREATE | O_RDWR));
    } else if what == 2 {
      close(open("grindir/../grindir/../b", O_CREATE | O_RDWR));
    } else if what == 3 {
      unlink("grindir/../a");
    } else if what == 4 {
      if chdir("grindir") != 0 {
        printf!("grind: chdir grindir failed\n");
        exit(1);
      }
      unlink("../b");
      chdir("/");
    } else if what == 5 {
      close(fd);
      fd = open("/grindir/../a", O_CREATE | O_RDWR);
    } else if what == 6 {
      close(fd);
      fd = open("/./grindir/./../b", O_CREATE | O_RDWR);
    } else if what == 7 {
      write(fd, buf);
    } else if what == 8 {
      read(fd, buf);
    } else if what == 9 {
      mkdir("grindir/../a");
      close(open("a/../a/./a", O_CREATE | O_RDWR));
      unlink("a/a");
    } else if what == 10 {
      mkdir("/../b");
      close(open("grindir/../b/b", O_CREATE | O_RDWR));
      unlink("b/b");
    } else if what == 11 {
      unlink("b");
      link("../grindir/./../a", "../b");
    } else if what == 12 {
      unlink("../grindir/../a");
      link(".././b", "/grindir/../a");
    } else if what == 13 {
      let pid = fork();
      if pid == 0 {
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      wait(None);
    } else if what == 14 {
      let pid = fork();
      if pid == 0 {
        fork();
        fork();
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      wait(None);
    } else if what == 15 {
      sbrk(6011);
    } else if what == 16 {
      if sbrk(0) > break0 {
        sbrk(-((sbrk(0) as usize - break0 as usize) as i32));
      }
    } else if what == 17 {
      let pid = fork();
      if pid == 0 {
        close(open("a", O_CREATE | O_RDWR));
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      if chdir("../grindir/..") != 0 {
        printf!("grind: chdir failed\n");
        exit(1);
      }
      kill(pid);
      wait(None);
    } else if what == 18 {
      let pid = fork();
      if pid == 0 {
        kill(getpid());
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      wait(None);
    } else if what == 19 {
      let mut fds = [0i32; 2];
      if pipe(&mut fds) < 0 {
        printf!("grind: pipe failed\n");
        exit(1);
      }
      let pid = fork();
      if pid == 0 {
        fork();
        fork();
        if write(fds[1], b"x") != 1 {
          printf!("grind: pipe write failed\n");
        }
        let mut c = [0u8; 1];
        if read(fds[0], &mut c) != 1 {
          printf!("grind: pipe read failed\n");
        }
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      close(fds[0]);
      close(fds[1]);
      wait(None);
    } else if what == 20 {
      let pid = fork();
      if pid == 0 {
        unlink("a");
        mkdir("a");
        chdir("a");
        unlink("../a");
        let _fd = open("x", O_CREATE | O_RDWR);
        unlink("x");
        exit(0);
      } else if pid < 0 {
        printf!("grind: fork failed\n");
        exit(1);
      }
      wait(None);
    } else if what == 21 {
      unlink("c");
      // should always succeed. check that there are free i-nodes,
      // file descriptors, blocks.
      let fd1 = open("c", O_CREATE | O_RDWR);
      if fd1 < 0 {
        printf!("grind: create c failed\n");
        exit(1);
      }
      if write(fd1, b"x") != 1 {
        printf!("grind: write c failed\n");
        exit(1);
      }
      let mut st = Stat::default();
      if fstat(fd1, &mut st) != 0 {
        printf!("grind: fstat failed\n");
        exit(1);
      }
      if st.size != 1 {
        printf!("grind: fstat reports wrong size {}\n", st.size as i32);
        exit(1);
      }
      if st.ino > 200 {
        printf!("grind: fstat reports crazy i-number {}\n", st.ino);
        exit(1);
      }
      close(fd1);
      unlink("c");
    } else if what == 22 {
      // echo hi | cat
      let mut aa = [0i32; 2];
      let mut bb = [0i32; 2];
      if pipe(&mut aa) < 0 {
        fprintf!(2, "grind: pipe failed\n");
        exit(1);
      }
      if pipe(&mut bb) < 0 {
        fprintf!(2, "grind: pipe failed\n");
        exit(1);
      }
      let pid1 = fork();
      if pid1 == 0 {
        close(bb[0]);
        close(bb[1]);
        close(aa[0]);
        close(1);
        if dup(aa[1]) != 1 {
          fprintf!(2, "grind: dup failed\n");
          exit(1);
        }
        close(aa[1]);
        let args: [*const u8; 3] = [c"echo".as_ptr().cast(), c"hi".as_ptr().cast(), ptr::null()];
        exec0(c"grindir/../echo", &args);
        fprintf!(2, "grind: echo: not found\n");
        exit(2);
      } else if pid1 < 0 {
        fprintf!(2, "grind: fork failed\n");
        exit(3);
      }
      let pid2 = fork();
      if pid2 == 0 {
        close(aa[1]);
        close(bb[0]);
        close(0);
        if dup(aa[0]) != 0 {
          fprintf!(2, "grind: dup failed\n");
          exit(4);
        }
        close(aa[0]);
        close(1);
        if dup(bb[1]) != 1 {
          fprintf!(2, "grind: dup failed\n");
          exit(5);
        }
        close(bb[1]);
        let args: [*const u8; 2] = [c"cat".as_ptr().cast(), ptr::null()];
        exec0(c"/cat", &args);
        fprintf!(2, "grind: cat: not found\n");
        exit(6);
      } else if pid2 < 0 {
        fprintf!(2, "grind: fork failed\n");
        exit(7);
      }
      close(aa[0]);
      close(aa[1]);
      close(bb[1]);
      let mut buf = [0u8; 4];
      read(bb[0], &mut buf[0..1]);
      read(bb[0], &mut buf[1..2]);
      read(bb[0], &mut buf[2..3]);
      close(bb[0]);
      let mut st1 = 0;
      let mut st2 = 0;
      wait(Some(&mut st1));
      wait(Some(&mut st2));
      if st1 != 0 || st2 != 0 || cstr(&buf) != b"hi\n" {
        printf!("grind: exec pipeline failed {} {} \"{}\"\n", st1, st2, as_str(&buf));
        exit(1);
      }
    }
  }
}

fn iter() -> ! {
  unlink("a");
  unlink("b");

  let pid1 = fork();
  if pid1 < 0 {
    printf!("grind: fork failed\n");
    exit(1);
  }
  if pid1 == 0 {
    unsafe { RAND_NEXT ^= 31 };
    go(0);
  }

  let pid2 = fork();
  if pid2 < 0 {
    printf!("grind: fork failed\n");
    exit(1);
  }
  if pid2 == 0 {
    unsafe { RAND_NEXT ^= 7177 };
    go(1);
  }

  let mut st1 = -1;
  wait(Some(&mut st1));
  if st1 != 0 {
    kill(pid1);
    kill(pid2);
  }
  let mut st2 = -1;
  wait(Some(&mut st2));

  exit(0);
}

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  loop {
    let pid = fork();
    if pid == 0 {
      iter();
    }
    if pid > 0 {
      wait(None);
    }
    pause(20);
    unsafe { RAND_NEXT += 1 };
  }
}
