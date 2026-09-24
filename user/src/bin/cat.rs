#![no_std]
#![no_main]

use user::*;

static mut BUF: [u8; 512] = [0; 512];

fn cat(fd: i32) {
  let buf = unsafe { &mut *(&raw mut BUF) };
  let mut n;
  loop {
    n = read(fd, buf);
    if n <= 0 {
      break;
    }
    if write(1, &buf[..n as usize]) != n {
      fprintf!(2, "cat: write error\n");
      exit(1);
    }
  }
  if n < 0 {
    fprintf!(2, "cat: read error\n");
    exit(1);
  }
}

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() <= 1 {
    cat(0);
    exit(0);
  }

  for arg in &args[1..] {
    let fd = open(*arg, O_RDONLY);
    if fd < 0 {
      fprintf!(2, "cat: cannot open {}\n", arg);
      exit(1);
    }
    cat(fd);
    close(fd);
  }
  0
}
