#![no_std]
#![no_main]

use user::*;

static mut BUF: [u8; 512] = [0; 512];

fn wc(fd: i32, name: &str) {
  let buf = unsafe { &mut *(&raw mut BUF) };
  let (mut l, mut w, mut c) = (0, 0, 0);
  let mut inword = false;
  let mut n;
  loop {
    n = read(fd, buf);
    if n <= 0 {
      break;
    }
    for &ch in &buf[..n as usize] {
      c += 1;
      if ch == b'\n' {
        l += 1;
      }
      if b" \r\t\n\x0b".contains(&ch) {
        inword = false;
      } else if !inword {
        w += 1;
        inword = true;
      }
    }
  }
  if n < 0 {
    printf!("wc: read error\n");
    exit(1);
  }
  printf!("{} {} {} {}\n", l, w, c, name);
}

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() <= 1 {
    wc(0, "");
    exit(0);
  }

  for arg in &args[1..] {
    let fd = open(*arg, O_RDONLY);
    if fd < 0 {
      printf!("wc: cannot open {}\n", arg);
      exit(1);
    }
    wc(fd, arg);
    close(fd);
  }
  0
}
