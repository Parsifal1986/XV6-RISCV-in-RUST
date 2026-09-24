#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  for (i, a) in args.iter().enumerate().skip(1) {
    write(1, a.as_bytes());
    if i + 1 < args.len() {
      write(1, b" ");
    } else {
      write(1, b"\n");
    }
  }
  0
}
