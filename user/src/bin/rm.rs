#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() < 2 {
    fprintf!(2, "Usage: rm files...\n");
    exit(1);
  }

  for arg in &args[1..] {
    if unlink(*arg) < 0 {
      fprintf!(2, "rm: {} failed to delete\n", arg);
      break;
    }
  }
  0
}
