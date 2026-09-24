// usertests: see main.rs.
// Section with tests that take a fair bit of time.

use core::ptr;

use user::*;

use crate::*;

// directory that uses indirect blocks
pub fn bigdir(s: &str) {
  const N: i32 = 500;
  let mut name = [0u8; 10];

  unlink("bd");

  let fd = open("bd", O_CREATE);
  if fd < 0 {
    printf!("{}: bigdir create failed\n", s);
    exit(1);
  }
  close(fd);

  for i in 0..N {
    name[0] = b'x';
    name[1] = b'0' + (i / 64) as u8;
    name[2] = b'0' + (i % 64) as u8;
    name[3] = b'\0';
    if link("bd", &name) != 0 {
      printf!("{}: bigdir i={} link(bd, {}) failed\n", s, i, as_str(&name));
      exit(1);
    }
  }

  unlink("bd");
  for i in 0..N {
    name[0] = b'x';
    name[1] = b'0' + (i / 64) as u8;
    name[2] = b'0' + (i % 64) as u8;
    name[3] = b'\0';
    if unlink(&name) != 0 {
      printf!("{}: bigdir unlink failed", s);
      exit(1);
    }
  }
}

// concurrent writes to try to provoke deadlock in the virtio disk
// driver.
pub fn manywrites(s: &str) {
  let nchildren = 4;
  let howmany = 30; // increase to look for deadlock

  for ci in 0..nchildren {
    let pid = fork();
    if pid < 0 {
      printf!("fork failed\n");
      exit(1);
    }

    if pid == 0 {
      let name = [b'b', b'a' + ci as u8, b'\0'];
      unlink(&name);

      for _iters in 0..howmany {
        for _i in 0..ci + 1 {
          let fd = open(&name, O_CREATE | O_RDWR);
          if fd < 0 {
            printf!("{}: cannot create {}\n", s, as_str(&name));
            exit(1);
          }
          let sz = BUFSZ as i32;
          let cc = write(fd, buf());
          if cc != sz {
            printf!("{}: write({}) ret {}\n", s, sz, cc);
            exit(1);
          }
          close(fd);
        }
        unlink(&name);
      }

      unlink(&name);
      exit(0);
    }
  }

  for _ci in 0..nchildren {
    let mut st = 0;
    wait(Some(&mut st));
    if st != 0 {
      exit(st);
    }
  }
  exit(0);
}

// regression test. does write() with an invalid buffer pointer cause
// a block to be allocated for a file that is then not freed when the
// file is deleted? if the kernel has this bug, it will panic: balloc:
// out of blocks. assumed_free may need to be raised to be more than
// the number of free blocks. this test takes a long time.
pub fn badwrite(_s: &str) {
  let assumed_free = 600;

  unlink("junk");
  for _i in 0..assumed_free {
    let fd = open("junk", O_CREATE | O_WRONLY);
    if fd < 0 {
      printf!("open junk failed\n");
      exit(1);
    }
    raw::write(fd, 0xffffffffff_usize as *const u8, 1);
    close(fd);
    unlink("junk");
  }

  let fd = open("junk", O_CREATE | O_WRONLY);
  if fd < 0 {
    printf!("open junk failed\n");
    exit(1);
  }
  if write(fd, b"x") != 1 {
    printf!("write failed\n");
    exit(1);
  }
  close(fd);
  unlink("junk");

  exit(0);
}

// test the exec() code that cleans up if it runs out
// of memory. it's really a test that such a condition
// doesn't cause a panic.
pub fn execout(_s: &str) {
  for avail in 0..15 {
    let pid = fork();
    if pid < 0 {
      printf!("fork failed\n");
      exit(1);
    } else if pid == 0 {
      // allocate all of memory.
      loop {
        let a = sbrk(PGSIZE as i32);
        if a == SBRK_ERROR {
          break;
        }
        unsafe { ptr::write_volatile(a.add(PGSIZE - 1), 1) };
      }

      // free a few pages, in order to let exec() make some
      // progress.
      for _i in 0..avail {
        sbrk(-(PGSIZE as i32));
      }

      close(1);
      // use the raw exec() so that building argv doesn't need
      // malloc(), which would take back the pages just freed.
      let args: [*const u8; 3] = [c"echo".as_ptr().cast(), c"x".as_ptr().cast(), ptr::null()];
      raw::exec(c"echo".as_ptr().cast(), args.as_ptr());
      exit(0);
    } else {
      wait(None);
    }
  }

  exit(0);
}

// can the kernel tolerate running out of disk space?
pub fn diskfull(s: &str) {
  let mut done = false;

  unlink("diskfulldir");

  let mut fi = 0;
  while !done && b'0' as i32 + fi < 0o177 {
    let name = [b'b', b'i', b'g', b'0' + fi as u8, b'\0'];
    unlink(&name);
    let fd = open(&name, O_CREATE | O_RDWR | O_TRUNC);
    if fd < 0 {
      // oops, ran out of inodes before running out of blocks.
      printf!("{}: could not create file {}\n", s, as_str(&name));
      break;
    }
    for _i in 0..MAXFILE {
      let buf = [0u8; BSIZE];
      if write(fd, &buf) != BSIZE as i32 {
        done = true;
        close(fd);
        break;
      }
    }
    close(fd);
    fi += 1;
  }

  // now that there are no free blocks, test that dirlink()
  // merely fails (doesn't panic) if it can't extend
  // directory content. one of these file creations
  // is expected to fail.
  let nzz = 128;
  for i in 0..nzz {
    let name = [b'z', b'z', b'0' + (i / 32) as u8, b'0' + (i % 32) as u8, b'\0'];
    unlink(&name);
    let fd = open(&name, O_CREATE | O_RDWR | O_TRUNC);
    if fd < 0 {
      break;
    }
    close(fd);
  }

  // this mkdir() is expected to fail.
  if mkdir("diskfulldir") == 0 {
    printf!("{}: mkdir(diskfulldir) unexpectedly succeeded!\n", s);
  }

  unlink("diskfulldir");

  for i in 0..nzz {
    let name = [b'z', b'z', b'0' + (i / 32) as u8, b'0' + (i % 32) as u8, b'\0'];
    unlink(&name);
  }

  let mut i = 0;
  while b'0' as i32 + i < 0o177 {
    let name = [b'b', b'i', b'g', b'0' + i as u8, b'\0'];
    unlink(&name);
    i += 1;
  }
}

pub fn outofinodes(_s: &str) {
  let nzz = 32 * 32;
  for i in 0..nzz {
    let name = [b'z', b'z', b'0' + (i / 32) as u8, b'0' + (i % 32) as u8, b'\0'];
    unlink(&name);
    let fd = open(&name, O_CREATE | O_RDWR | O_TRUNC);
    if fd < 0 {
      // failure is eventually expected.
      break;
    }
    close(fd);
  }

  for i in 0..nzz {
    let name = [b'z', b'z', b'0' + (i / 32) as u8, b'0' + (i % 32) as u8, b'\0'];
    unlink(&name);
  }
}
