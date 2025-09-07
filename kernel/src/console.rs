use crate::file::{DEVSW, CONSOLE};
use crate::proc::{either_copyin, either_copyout, killed, myproc, sleep};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::uart::{uartinit, uartputc_sync, uartwrite};

pub const BASKSPACE:u64 = 0x100;

pub fn char2num(x:u8) -> u8 {
  x - ('@' as u8)
}

pub fn consputc(c:i32) {
  if c == BASKSPACE as i32 {
    uartputc_sync(0x08);
    uartputc_sync(' ' as u8);
    uartputc_sync(0x08);
  } else {
    uartputc_sync(c as u8);
  }
}

const INPUT_BUF_SIZE: usize = 128;

struct Console {
  lock: Spinlock,
  buf: [u8; INPUT_BUF_SIZE],
  r: u32,
  w: u32,
  e: u32,
}

impl Console {
  pub const fn new() -> Self {
    Console {
      lock: Spinlock::new(),
      buf: [b'\0'; INPUT_BUF_SIZE],
      r: 0,
      w: 0,
      e: 0
    }
  }
}

static mut CONS: Console = Console::new();

pub fn consolewrite(user_src: i32, src: u64, n: i32) -> i32{
  let mut buf: [u8;32] = [0x00; 32];
  let mut i = 0;

  while i < n {
    let mut nn  = size_of::<[u8; 32]>();

    if nn > (n - i).try_into().unwrap() {
      nn = (n - i).try_into().unwrap();
    }

    if either_copyin(buf.as_mut_ptr(), user_src, (src + 1) as *const u8, nn as u64) == -1 {
      break;
    }

    uartwrite(&mut buf, nn as i32);
    i += nn as i32;
  }

  i
}

pub fn consoleread(user_dst: i32, mut dst: u64, mut n: i32) -> i32 {
  let target: u32 = n as u32;
  let mut c: u8;
  let mut cbuf: u8;

  acquire(unsafe{ &mut CONS.lock });
  while n > 0 {
    while unsafe { CONS.r == CONS.w } {
      if killed(myproc().unwrap()) {
        release(unsafe{ &mut CONS.lock });
        return -1;
      }
      unsafe {
        sleep(&mut CONS.r as *mut _ as *mut u8, &mut CONS.lock);
      }

      c = unsafe { CONS.buf[CONS.r as usize % INPUT_BUF_SIZE] } as u8;
    
      if c == char2num('D' as u8) {
        if n < target as i32 {
          unsafe {
            CONS.r -= 1;
          }
        }
        break;
      }

      cbuf = c as u8;

      if either_copyout(user_dst, dst as *const u8, (&cbuf) as *const u8, 1) == -1 {
        break;
      }

      dst += 1;

      n -= 1;

      if c == '\n' as u8 {
        break;
      }
    }
  }
  release(unsafe { &mut CONS.lock });

  return target as i32 - n;
}

pub fn consoleinit() {
  initlock(unsafe { &mut CONS.lock }, Some("console".as_bytes()));

  uartinit();

  unsafe {
    DEVSW[CONSOLE as usize].read = consoleread;
    DEVSW[CONSOLE as usize].write = consolewrite;
  }
}