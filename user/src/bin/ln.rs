#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() != 3 {
    fprintf!(2, "Usage: ln old new\n");
    exit(1);
  }
  if link(args[1], args[2]) < 0 {
    fprintf!(2, "link {} {}: failed\n", args[1], args[2]);
  }
  0
}
