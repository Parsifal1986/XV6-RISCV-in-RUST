// Simple grep.  Only supports ^ . * $ operators.

#![no_std]
#![no_main]

use user::*;

static mut BUF: [u8; 1024] = [0; 1024];

fn grep(pattern: &[u8], fd: i32) {
  let buf = unsafe { &mut *(&raw mut BUF) };
  let mut m = 0;
  loop {
    let len = buf.len();
    let n = read(fd, &mut buf[m..len - 1]);
    if n <= 0 {
      break;
    }
    m += n as usize;
    buf[m] = 0;
    let mut p = 0;
    while let Some(q) = buf[p..m].iter().position(|&c| c == b'\n') {
      let q = p + q;
      if matches(pattern, &buf[p..q]) {
        write(1, &buf[p..q + 1]);
      }
      p = q + 1;
    }
    if m > 0 {
      m -= p;
      buf.copy_within(p..p + m, 0);
    }
  }
}

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() <= 1 {
    fprintf!(2, "usage: grep pattern [file ...]\n");
    exit(1);
  }
  let pattern = args[1].as_bytes();

  if args.len() <= 2 {
    grep(pattern, 0);
    exit(0);
  }

  for arg in &args[2..] {
    let fd = open(*arg, O_RDONLY);
    if fd < 0 {
      printf!("grep: cannot open {}\n", arg);
      exit(1);
    }
    grep(pattern, fd);
    close(fd);
  }
  0
}

// Regexp matcher from Kernighan & Pike,
// The Practice of Programming, Chapter 9, or
// https://www.cs.princeton.edu/courses/archive/spr09/cos333/beautiful.html

// the byte at i, or 0 past the end, like a C string.
fn at(s: &[u8], i: usize) -> u8 {
  if i < s.len() {
    s[i]
  } else {
    0
  }
}

fn matches(re: &[u8], text: &[u8]) -> bool {
  if at(re, 0) == b'^' {
    return matchhere(&re[1..], text);
  }
  let mut t = 0;
  loop {
    // must look at empty string
    if matchhere(re, &text[t..]) {
      return true;
    }
    if t >= text.len() {
      return false;
    }
    t += 1;
  }
}

// matchhere: search for re at beginning of text
fn matchhere(re: &[u8], text: &[u8]) -> bool {
  if at(re, 0) == 0 {
    return true;
  }
  if at(re, 1) == b'*' {
    return matchstar(re[0], &re[2..], text);
  }
  if at(re, 0) == b'$' && at(re, 1) == 0 {
    return at(text, 0) == 0;
  }
  if at(text, 0) != 0 && (re[0] == b'.' || re[0] == text[0]) {
    return matchhere(&re[1..], &text[1..]);
  }
  false
}

// matchstar: search for c*re at beginning of text
fn matchstar(c: u8, re: &[u8], text: &[u8]) -> bool {
  let mut t = 0;
  loop {
    // a * matches zero or more instances
    if matchhere(re, &text[t..]) {
      return true;
    }
    if !(t < text.len() && (text[t] == c || c == b'.')) {
      return false;
    }
    t += 1;
  }
}
