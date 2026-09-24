// Test that fork fails gracefully.
// Tiny executable so that the limit can be filling the proc table.

#![no_std]
#![no_main]

use user::*;

const N: i32 = 1000;

fn print(s: &str) {
  write(1, s.as_bytes());
}

fn forktest() {
  print("fork test\n");

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

  if n == N {
    print("fork claimed to work N times!\n");
    exit(1);
  }

  while n > 0 {
    if wait(None) < 0 {
      print("wait stopped early\n");
      exit(1);
    }
    n -= 1;
  }

  if wait(None) != -1 {
    print("wait got too many\n");
    exit(1);
  }

  print("fork test OK\n");
}

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  forktest();
  0
}
