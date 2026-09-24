// Demonstrate that moving the "acquire" in iderw after the loop that
// appends to the idequeue results in a race.

// For this to work, you should also add a spin within iderw's
// idequeue traversal loop.  Adding the following demonstrated a panic
// after about 5 runs of stressfs in QEMU on a 2.1GHz CPU:
//    for (i = 0; i < 40000; i++)
//      asm volatile("");

#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  let mut path = *b"stressfs0";
  let mut data = [b'a'; 512];

  printf!("stressfs starting\n");

  let mut i = 0;
  while i < 4 {
    if fork() > 0 {
      break;
    }
    i += 1;
  }

  printf!("write {}\n", i);

  path[8] += i as u8;
  let fd = open(&path, O_CREATE | O_RDWR);
  for _ in 0..20 {
    write(fd, &data);
  }
  close(fd);

  printf!("read\n");

  let fd = open(&path, O_RDONLY);
  for _ in 0..20 {
    read(fd, &mut data);
  }
  close(fd);

  wait(None);

  0
}
