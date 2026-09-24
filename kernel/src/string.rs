// Helpers for NUL-terminated byte strings, as used by
// the file system and exec.

// length of a NUL-terminated string.
pub fn strlen(s: *const u8) -> usize {
  let mut n = 0;
  unsafe {
    while *s.add(n) != 0 {
      n += 1;
    }
  }
  n
}

// the part of buf before the first NUL (or all of buf).
pub fn cstr(buf: &[u8]) -> &[u8] {
  match buf.iter().position(|&c| c == 0) {
    Some(n) => &buf[..n],
    None => buf,
  }
}

// compare at most n bytes of two strings, treating the end
// of a slice like a NUL terminator.
pub fn strncmp(p: &[u8], q: &[u8], n: usize) -> i32 {
  for i in 0..n {
    let a = if i < p.len() { p[i] } else { 0 };
    let b = if i < q.len() { q[i] } else { 0 };
    if a != b {
      return a as i32 - b as i32;
    }
    if a == 0 {
      return 0;
    }
  }
  0
}

// Like strncpy but guaranteed to NUL-terminate.
pub fn safestrcpy(dst: &mut [u8], src: &[u8]) {
  if dst.is_empty() {
    return;
  }
  let src = cstr(src);
  let n = core::cmp::min(dst.len() - 1, src.len());
  dst[..n].copy_from_slice(&src[..n]);
  dst[n] = 0;
}
