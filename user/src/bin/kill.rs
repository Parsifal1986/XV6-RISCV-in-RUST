#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() < 2 {
    fprintf!(2, "usage: kill pid...\n");
    exit(1);
  }
  for arg in &args[1..] {
    kill(atoi(*arg));
  }
  0
}
