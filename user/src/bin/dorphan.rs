// Create an orphaned directory and check if test-xv6.py recovers it.

#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  let s = args[0];

  if mkdir("dd") != 0 {
    printf!("{}: mkdir dd failed\n", s);
    exit(1);
  }
  if chdir("dd") != 0 {
    printf!("{}: chdir dd failed\n", s);
    exit(1);
  }
  if unlink("../dd") < 0 {
    printf!("{}: unlink failed\n", s);
    exit(1);
  }
  printf!("wait for kill and reclaim\n");
  // sit around until killed
  loop {
    pause(1000);
  }
}
