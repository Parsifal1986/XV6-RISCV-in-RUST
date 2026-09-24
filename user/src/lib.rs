// The xv6 user-level library: system calls, printf,
// malloc, and a few string helpers.
//
// A user program looks like:
//
//   #![no_std]
//   #![no_main]
//   use user::*;
//
//   #[no_mangle]
//   fn main(args: &[&str]) -> i32 {
//     printf!("hello {}\n", args[0]);
//     0
//   }
//
// main's return value becomes the process's exit status.

#![no_std]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::redundant_closure)]

extern crate alloc;

#[macro_use]
pub mod printf;
pub mod syscall;
pub mod ulib;
pub mod umalloc;

pub use printf::*;
pub use syscall::raw;
pub use ulib::*;
pub use umalloc::{free, malloc};

use core::panic::PanicInfo;

pub const MAXARG: usize = 32; // max exec arguments
pub const MAXPATH: usize = 128; // maximum file path name
pub const PGSIZE: usize = 4096;
pub const USERSTACK: usize = 4; // user stack pages (see kernel/src/param.rs)

// open() modes
pub const O_RDONLY: i32 = 0x000;
pub const O_WRONLY: i32 = 0x001;
pub const O_RDWR: i32 = 0x002;
pub const O_CREATE: i32 = 0x200;
pub const O_TRUNC: i32 = 0x400;

// file types
pub const T_DIR: i16 = 1; // Directory
pub const T_FILE: i16 = 2; // File
pub const T_DEVICE: i16 = 3; // Device

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Stat {
  pub dev: i32,   // File system's disk device
  pub ino: u32,   // Inode number
  pub typ: i16,   // Type of file
  pub nlink: i16, // Number of links to file
  pub size: u64,  // Size of file in bytes
}

// Directory is a file containing a sequence of dirent structures.
pub const DIRSIZ: usize = 14;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Dirent {
  pub inum: u16,
  pub name: [u8; DIRSIZ],
}

pub const SBRK_EAGER: i32 = 1;
pub const SBRK_LAZY: i32 = 2;
pub const SBRK_ERROR: *mut u8 = usize::MAX as *mut u8;

extern "Rust" {
  // provided by each user program.
  fn main(args: &[&str]) -> i32;
}

// exec() starts the program here, with argc in a0 and argv in a1.
#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: i32, argv: *const *const u8) -> ! {
  let mut args: [&str; MAXARG] = [""; MAXARG];
  let argc = core::cmp::min(argc.max(0) as usize, MAXARG);
  for (i, arg) in args.iter_mut().enumerate().take(argc) {
    unsafe {
      let p = *argv.add(i);
      let bytes = core::slice::from_raw_parts(p, strlen_ptr(p));
      *arg = core::str::from_utf8_unchecked(bytes);
    }
  }
  let r = unsafe { main(&args[..argc]) };
  exit(r);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
  fprintf!(2, "panic: {}\n", info.message());
  exit(1);
}
