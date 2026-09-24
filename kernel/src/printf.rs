//
// formatted console output -- printf, panic.
//

use core::fmt;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::console::consputc;
use crate::spinlock::{acquire, initlock, release, Spinlock};

pub static PANICKING: AtomicBool = AtomicBool::new(false); // printing a panic message
pub static PANICKED: AtomicBool = AtomicBool::new(false);  // spinning forever at end of a panic

// lock to avoid interleaving concurrent printf's.
static mut PR: Spinlock = Spinlock::new();

// print to the console, like the C printf, but with Rust formatting:
//   printf!("pid {} name {}\n", pid, name);
#[macro_export]
macro_rules! printf {
  ($($arg:tt)*) => {
    $crate::printf::printf(format_args!($($arg)*))
  };
}

struct Writer;

impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    for c in s.bytes() {
      consputc(c as i32);
    }
    Ok(())
  }
}

// Print to the console.
pub fn printf(args: fmt::Arguments) -> i32 {
  let locking = !PANICKING.load(Ordering::Relaxed);

  if locking {
    acquire(&raw mut PR);
  }

  let _ = fmt::write(&mut Writer, args);

  if locking {
    release(&raw mut PR);
  }

  0
}

pub fn panic(msg: &str) -> ! {
  PANICKING.store(true, Ordering::Relaxed);
  printf!("panic: {}\n", msg);
  PANICKED.store(true, Ordering::Relaxed); // freeze uart output from other CPUs
  loop {
    core::hint::spin_loop();
  }
}

// Rust-level panics (e.g. failed bounds checks) end up here.
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
  PANICKING.store(true, Ordering::Relaxed);
  if let Some(location) = info.location() {
    printf!("panic: {} ({}:{})\n", info.message(), location.file(), location.line());
  } else {
    printf!("panic: {}\n", info.message());
  }
  PANICKED.store(true, Ordering::Relaxed);
  loop {
    core::hint::spin_loop();
  }
}

pub fn printfinit() {
  initlock(&raw mut PR, "pr");
}
