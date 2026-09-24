// Create a zombie process that
// must be reparented at exit.

#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  if fork() > 0 {
    pause(5); // Let child exit before parent.
  }
  0
}
