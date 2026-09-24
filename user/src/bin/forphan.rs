// Create an orphaned file and check if test-xv6.py recovers it.

#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  let s = args[0];
  let mut st = Stat::default();
  let ff = "file0";

  let fd = open(ff, O_CREATE | O_WRONLY);
  if fd < 0 {
    printf!("{}: open failed\n", s);
    exit(1);
  }
  if fstat(fd, &mut st) < 0 {
    fprintf!(2, "{}: cannot stat {}\n", s, "ff");
    exit(1);
  }
  if unlink(ff) < 0 {
    printf!("{}: unlink failed\n", s);
    exit(1);
  }
  if open(ff, O_RDONLY) != -1 {
    printf!("{}: open successed\n", s);
    exit(1);
  }
  printf!("wait for kill and reclaim {}\n", st.ino);
  // sit around until killed
  loop {
    pause(1000);
  }
}
