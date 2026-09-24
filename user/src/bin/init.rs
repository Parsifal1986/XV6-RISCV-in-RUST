// init: The initial user-level program

#![no_std]
#![no_main]

use user::*;

const CONSOLE: i16 = 1;

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  if open("console", O_RDWR) < 0 {
    mknod("console", CONSOLE, 0);
    open("console", O_RDWR);
  }
  dup(0); // stdout
  dup(0); // stderr

  loop {
    printf!("init: starting sh\n");
    let pid = fork();
    if pid < 0 {
      printf!("init: fork failed\n");
      exit(1);
    }
    if pid == 0 {
      exec("sh", &["sh"]);
      printf!("init: exec sh failed\n");
      exit(1);
    }

    loop {
      // this call to wait() returns if the shell exits,
      // or if a parentless process exits.
      let wpid = wait(None);
      if wpid == pid {
        // the shell exited; restart it.
        break;
      } else if wpid < 0 {
        printf!("init: wait returned an error\n");
        exit(1);
      } else {
        // it was a parentless process; do nothing.
      }
    }
  }
}
