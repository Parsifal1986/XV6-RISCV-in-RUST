use core::fmt;
use core::mem::size_of;
use core::panic::PanicInfo;

use crate::console::consputc;
use crate::spinlock::{acquire, initlock, release, Spinlock};

pub static mut PANICKING:i32 = 0;
pub static mut PANICKED:i32 = 0;

static mut PR: Spinlock = Spinlock::new();
static DIGITS: &[u8] = b"0123456789abcdef";

fn printint(xx: i64, base: u64, mut sign: bool) {
  let mut buf: [u8; 20] = [0; 20];
  let mut i: i32 = 0;
  let x: u64;

  x = if sign && {sign = xx < 0;xx < 0} {
    -xx as u64
  } else {
    xx as u64
  };

  loop {
    buf[i as usize] = DIGITS[(x % base) as usize];
    i += 1;
    if x / base == 0 {
      break;
    }
  }

  if sign {
    buf[i as usize] = b'-';
    i += 1;
  }

  while {i -= 1; i >= 0} {
    consputc(buf[i as usize] as i32);
  }
}

fn printptr(x: u64) {
  consputc('0' as i32);
  consputc('x' as i32);
  for _ in 0..(size_of::<u64>() * 2 - 1){
    let x = x << 4;
    consputc(DIGITS[((x >> (size_of::<u64>() * 8 - 4)) & 0xf) as usize] as i32);
  }
}

pub fn printf(args: fmt::Arguments) -> i32 {
  if unsafe { PANICKING == 0 } {
    acquire(unsafe { &mut PR });
  }

  let mut writer = Writer{};

  fmt::write(&mut writer, args).unwrap();

  if unsafe { PANICKING == 0 } {
    release(unsafe { &mut PR });
  }

  0
}

struct Writer;

impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    for c in s.chars() {
      consputc(c as i32);
    }
    Ok(())
  }
}

pub fn panic(msg: &str) -> ! {
  unsafe { PANICKING = 1; }
  printf(format_args!("panic: {}\n", msg));
  unsafe { PANICKED = 1; }
  loop{}
}

// #[panic_handler]
// fn panic_handler(info: &PanicInfo) -> ! {
//     unsafe { PANICKING = 1; }
//     if let Some(location) = info.location() {
//         printf(
//             "panic at {}:{}: {}\n",
//             location.file(),
//             location.line(),
//             info.message().unwrap_or_else(|| &"Unknown panic").to_string(),
//         );
//     } else {
//         printf("panic: Unknown location\n");
//     }
//     printf("panic occurred!\n");
//     loop {}
// }

pub fn printfinit() {
  unsafe {
    initlock(&mut PR, Some("pr".as_bytes()));
  }
}