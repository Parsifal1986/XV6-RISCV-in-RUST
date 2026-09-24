// Formatted output to a file descriptor, with Rust formatting:
//
//   printf!("{} {}\n", a, b);       // to stdout (fd 1)
//   fprintf!(2, "error: {}\n", e);  // to any fd

use core::fmt;

use crate::syscall::raw;

// accumulate output and write() it in chunks.
pub struct FdWriter {
  fd: i32,
  buf: [u8; 128],
  n: usize,
}

impl FdWriter {
  pub fn new(fd: i32) -> Self {
    FdWriter { fd, buf: [0; 128], n: 0 }
  }

  pub fn flush(&mut self) {
    if self.n > 0 {
      raw::write(self.fd, self.buf.as_ptr(), self.n as i32);
      self.n = 0;
    }
  }
}

impl fmt::Write for FdWriter {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    for &c in s.as_bytes() {
      if self.n == self.buf.len() {
        self.flush();
      }
      self.buf[self.n] = c;
      self.n += 1;
    }
    Ok(())
  }
}

pub fn fprintf(fd: i32, args: fmt::Arguments) {
  let mut w = FdWriter::new(fd);
  let _ = fmt::write(&mut w, args);
  w.flush();
}

#[macro_export]
macro_rules! printf {
  ($($arg:tt)*) => {
    $crate::printf::fprintf(1, format_args!($($arg)*))
  };
}

#[macro_export]
macro_rules! fprintf {
  ($fd:expr, $($arg:tt)*) => {
    $crate::printf::fprintf($fd, format_args!($($arg)*))
  };
}
