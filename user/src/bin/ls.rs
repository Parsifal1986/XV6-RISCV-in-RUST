#![no_std]
#![no_main]

use core::mem::size_of;

use user::*;

// Return blank-padded name of the last path element.
fn fmtname<'a>(path: &'a [u8], buf: &'a mut [u8; DIRSIZ]) -> &'a str {
  // Find first character after last slash.
  let p = match path.iter().rposition(|&c| c == b'/') {
    Some(i) => &path[i + 1..],
    None => path,
  };

  // Return blank-padded name.
  if p.len() >= DIRSIZ {
    return as_str(p);
  }
  buf[..p.len()].copy_from_slice(p);
  buf[p.len()..].fill(b' ');
  unsafe { core::str::from_utf8_unchecked(&buf[..]) }
}

fn ls(path: &str) {
  let mut buf = [0u8; 512];
  let mut name = [0u8; DIRSIZ];
  let mut de = Dirent::default();
  let mut st = Stat::default();

  let fd = open(path, O_RDONLY);
  if fd < 0 {
    fprintf!(2, "ls: cannot open {}\n", path);
    return;
  }

  if fstat(fd, &mut st) < 0 {
    fprintf!(2, "ls: cannot stat {}\n", path);
    close(fd);
    return;
  }

  match st.typ {
    T_DEVICE | T_FILE => {
      printf!("{} {} {} {}\n", fmtname(path.as_bytes(), &mut name), st.typ, st.ino, st.size as i32);
    }
    T_DIR => {
      if path.len() + 1 + DIRSIZ + 1 > buf.len() {
        printf!("ls: path too long\n");
      } else {
        buf[..path.len()].copy_from_slice(path.as_bytes());
        let p = path.len() + 1;
        buf[p - 1] = b'/';
        let desz = size_of::<Dirent>();
        loop {
          let debuf = unsafe { core::slice::from_raw_parts_mut(&raw mut de as *mut u8, desz) };
          if read(fd, debuf) != desz as i32 {
            break;
          }
          if de.inum == 0 {
            continue;
          }
          buf[p..p + DIRSIZ].copy_from_slice(&de.name);
          buf[p + DIRSIZ] = 0;
          let full = cstr(&buf);
          if stat(full, &mut st) < 0 {
            printf!("ls: cannot stat {}\n", as_str(full));
            continue;
          }
          printf!("{} {} {} {}\n", fmtname(full, &mut name), st.typ, st.ino, st.size as i32);
        }
      }
    }
    _ => {}
  }
  close(fd);
}

#[no_mangle]
fn main(args: &[&str]) -> i32 {
  if args.len() < 2 {
    ls(".");
    exit(0);
  }
  for arg in &args[1..] {
    ls(arg);
  }
  0
}
