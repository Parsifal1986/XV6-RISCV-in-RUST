// Stress xv6 logging system by having several processes writing
// concurrently to their own file (e.g., logstress f1 f2 f3 f4)

#![no_std]
#![no_main]

use user::*;

const N: i32 = 250;
const SZ: usize = 2000;

static mut BUF: [u8; SZ] = [0; SZ];

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  let buf = unsafe { &mut *(&raw mut BUF) };

  for i in 1..args.len() {
    let pid1 = fork();
    if pid1 < 0 {
      printf!("{}: fork failed\n", args[0]);
      exit(1);
    }
    if pid1 == 0 {
      let fd = open(args[i], O_CREATE | O_RDWR);
      if fd < 0 {
        printf!("{}: create {} failed\n", args[0], args[i]);
        exit(1);
      }
      buf.fill(b'0' + i as u8);
      for _ in 0..N {
        let n = write(fd, buf);
        if n != SZ as i32 {
          printf!("write failed {}\n", n);
          exit(1);
        }
      }
      exit(0);
    }
  }

  let mut xstatus = 0;
  for _ in 1..args.len() {
    wait(Some(&mut xstatus));
    if xstatus != 0 {
      exit(xstatus);
    }
  }
  0
}
