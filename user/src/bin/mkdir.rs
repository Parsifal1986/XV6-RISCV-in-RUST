#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() < 2 {
    fprintf!(2, "Usage: mkdir files...\n");
    exit(1);
  }

  for arg in &args[1..] {
    if mkdir(*arg) < 0 {
      fprintf!(2, "mkdir: {} failed to create\n", arg);
      break;
    }
  }
  0
}
