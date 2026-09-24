//
// Console input and output, to the uart.
// Reads are line at a time.
// Implements special input characters:
//   newline -- end of line
//   control-h -- backspace
//   control-u -- kill line
//   control-d -- end of file
//   control-p -- print process list
//

use crate::file::{CONSOLE, DEVSW};
use crate::proc::{either_copyin, either_copyout, killed, myproc, procdump, sleep, wakeup};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::uart::{uartinit, uartputc_sync, uartwrite};

const BACKSPACE: i32 = 0x100;

// Control-x
const fn C(x: u8) -> i32 {
  (x - b'@') as i32
}

//
// send one character to the uart.
// called by printf(), and to echo input characters,
// but not from write().
//
pub fn consputc(c: i32) {
  if c == BACKSPACE {
    // if the user typed backspace, overwrite with a space.
    uartputc_sync(0x08);
    uartputc_sync(b' ');
    uartputc_sync(0x08);
  } else {
    uartputc_sync(c as u8);
  }
}

const INPUT_BUF_SIZE: usize = 128;

struct Console {
  lock: Spinlock,

  // input
  buf: [u8; INPUT_BUF_SIZE],
  r: u32, // Read index
  w: u32, // Write index
  e: u32, // Edit index
}

static mut CONS: Console = Console {
  lock: Spinlock::new(),
  buf: [0; INPUT_BUF_SIZE],
  r: 0,
  w: 0,
  e: 0,
};

//
// user write()s to the console go here.
//
fn consolewrite(user_src: bool, src: u64, n: i32) -> i32 {
  let mut buf = [0u8; 32];
  let mut i = 0;

  while i < n {
    let mut nn = buf.len() as i32;
    if nn > n - i {
      nn = n - i;
    }
    if either_copyin(buf.as_mut_ptr(), user_src, src + i as u64, nn as u64) == -1 {
      break;
    }
    uartwrite(&buf[..nn as usize]);
    i += nn;
  }

  i
}

//
// user read()s from the console go here.
// copy (up to) a whole input line to dst.
// user_dst indicates whether dst is a user
// or kernel address.
//
fn consoleread(user_dst: bool, mut dst: u64, mut n: i32) -> i32 {
  let target = n;

  unsafe {
    acquire(&raw mut CONS.lock);
    while n > 0 {
      // wait until interrupt handler has put some
      // input into CONS.buf.
      while core::ptr::read_volatile(&raw const CONS.r) == core::ptr::read_volatile(&raw const CONS.w) {
        if killed(myproc()) {
          release(&raw mut CONS.lock);
          return -1;
        }
        sleep(&raw const CONS.r as *const u8, &raw mut CONS.lock);
      }

      let c = CONS.buf[CONS.r as usize % INPUT_BUF_SIZE] as i32;
      CONS.r = CONS.r.wrapping_add(1);

      if c == C(b'D') {
        // end-of-file
        if n < target {
          // Save ^D for next time, to make sure
          // caller gets a 0-byte result.
          CONS.r = CONS.r.wrapping_sub(1);
        }
        break;
      }

      // copy the input byte to the user-space buffer.
      let cbuf = c as u8;
      if either_copyout(user_dst, dst, &cbuf, 1) == -1 {
        break;
      }

      dst += 1;
      n -= 1;

      if c == b'\n' as i32 {
        // a whole line has arrived, return to
        // the user-level read().
        break;
      }
    }
    release(&raw mut CONS.lock);
  }

  target - n
}

//
// the console input interrupt handler.
// uartintr() calls this for input character.
// do erase/kill processing, append to CONS.buf,
// wake up consoleread() if a whole line has arrived.
//
pub fn consoleintr(mut c: i32) {
  unsafe {
    acquire(&raw mut CONS.lock);

    if c == C(b'P') {
      // Print process list.
      procdump();
    } else if c == C(b'U') {
      // Kill line.
      while CONS.e != CONS.w && CONS.buf[CONS.e.wrapping_sub(1) as usize % INPUT_BUF_SIZE] != b'\n' {
        CONS.e = CONS.e.wrapping_sub(1);
        consputc(BACKSPACE);
      }
    } else if c == C(b'H') || c == 0x7f {
      // Backspace or Delete key
      if CONS.e != CONS.w {
        CONS.e = CONS.e.wrapping_sub(1);
        consputc(BACKSPACE);
      }
    } else if c != 0 && CONS.e.wrapping_sub(CONS.r) < INPUT_BUF_SIZE as u32 {
      c = if c == b'\r' as i32 { b'\n' as i32 } else { c };

      // echo back to the user.
      consputc(c);

      // store for consumption by consoleread().
      CONS.buf[CONS.e as usize % INPUT_BUF_SIZE] = c as u8;
      CONS.e = CONS.e.wrapping_add(1);

      if c == b'\n' as i32 || c == C(b'D') || CONS.e.wrapping_sub(CONS.r) == INPUT_BUF_SIZE as u32 {
        // wake up consoleread() if a whole line (or end-of-file)
        // has arrived.
        CONS.w = CONS.e;
        wakeup(&raw const CONS.r as *const u8);
      }
    }

    release(&raw mut CONS.lock);
  }
}

pub fn consoleinit() {
  unsafe {
    initlock(&raw mut CONS.lock, "cons");
  }

  uartinit();

  // connect read and write system calls
  // to consoleread and consolewrite.
  unsafe {
    DEVSW[CONSOLE].read = Some(consoleread);
    DEVSW[CONSOLE].write = Some(consolewrite);
  }
}
