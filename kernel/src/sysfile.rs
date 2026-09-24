//
// File-system system calls.
// Mostly argument checking, since we don't trust
// user code, and calls into file.rs and fs.rs.
//

use core::ptr::{null, null_mut};

use crate::exec::kexec;
use crate::fcntl::{O_CREATE, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};
use crate::file::{filealloc, fileclose, filedup, fileread, filestat, filewrite, File, FileType, Inode};
use crate::fs::{
  dirlink, dirlookup, ialloc, ilock, iput, itrunc, iunlock, iunlockput, iupdate, namecmp, namei, nameiparent, readi,
  writei, Dirent, DIRSIZ, NLINK_MAX,
};
use crate::kalloc::{kalloc, kfree};
use crate::log::{begin_op, end_op};
use crate::param::{MAXARG, MAXPATH, NDEV, NOFILE};
use crate::pipe::pipealloc;
use crate::printf::panic;
use crate::proc::myproc;
use crate::riscv::PGSIZE;
use crate::stat::{T_DEVICE, T_DIR, T_FILE};
use crate::string::cstr;
use crate::syscall::{argaddr, argint, argstr, fetchaddr, fetchstr};
use crate::vm::copyout;

// Fetch the nth word-sized system call argument as a file descriptor
// and return both the descriptor and the corresponding struct file.
fn argfd(n: i32, pfd: Option<&mut i32>, pf: Option<&mut *mut File>) -> i32 {
  let mut fd: i32 = 0;

  argint(n, &mut fd);
  if fd < 0 || fd >= NOFILE as i32 {
    return -1;
  }
  let f = unsafe { (*myproc()).ofile[fd as usize] };
  if f.is_null() {
    return -1;
  }
  if let Some(pfd) = pfd {
    *pfd = fd;
  }
  if let Some(pf) = pf {
    *pf = f;
  }
  0
}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fdalloc(f: *mut File) -> i32 {
  let p = myproc();
  for fd in 0..NOFILE {
    unsafe {
      if (*p).ofile[fd].is_null() {
        (*p).ofile[fd] = f;
        return fd as i32;
      }
    }
  }
  -1
}

pub fn sys_dup() -> u64 {
  let mut f: *mut File = null_mut();

  if argfd(0, None, Some(&mut f)) < 0 {
    return u64::MAX;
  }
  let fd = fdalloc(f);
  if fd < 0 {
    return u64::MAX;
  }
  filedup(f);
  fd as u64
}

pub fn sys_read() -> u64 {
  let mut f: *mut File = null_mut();
  let mut n: i32 = 0;
  let mut p: u64 = 0;

  argaddr(1, &mut p);
  argint(2, &mut n);
  if argfd(0, None, Some(&mut f)) < 0 {
    return u64::MAX;
  }
  fileread(f, p, n) as u64
}

pub fn sys_write() -> u64 {
  let mut f: *mut File = null_mut();
  let mut n: i32 = 0;
  let mut p: u64 = 0;

  argaddr(1, &mut p);
  argint(2, &mut n);
  if argfd(0, None, Some(&mut f)) < 0 {
    return u64::MAX;
  }
  filewrite(f, p, n) as u64
}

pub fn sys_close() -> u64 {
  let mut fd: i32 = 0;
  let mut f: *mut File = null_mut();

  if argfd(0, Some(&mut fd), Some(&mut f)) < 0 {
    return u64::MAX;
  }
  unsafe {
    (*myproc()).ofile[fd as usize] = null_mut();
  }
  fileclose(f);
  0
}

pub fn sys_fstat() -> u64 {
  let mut f: *mut File = null_mut();
  let mut st: u64 = 0; // user pointer to struct stat

  argaddr(1, &mut st);
  if argfd(0, None, Some(&mut f)) < 0 {
    return u64::MAX;
  }
  filestat(f, st) as u64
}

// Create the path new as a link to the same inode as old.
pub fn sys_link() -> u64 {
  let mut name = [0u8; DIRSIZ];
  let mut new = [0u8; MAXPATH];
  let mut old = [0u8; MAXPATH];

  if argstr(0, &mut old) < 0 || argstr(1, &mut new) < 0 {
    return u64::MAX;
  }

  begin_op();
  let ip = namei(cstr(&old));
  if ip.is_null() {
    end_op();
    return u64::MAX;
  }

  ilock(ip);
  unsafe {
    if (*ip).typ == T_DIR {
      iunlockput(ip);
      end_op();
      return u64::MAX;
    }
    if (*ip).nlink >= NLINK_MAX {
      iunlockput(ip);
      end_op();
      return u64::MAX;
    }

    (*ip).nlink += 1;
    iupdate(ip);
    iunlock(ip);

    let dp = nameiparent(cstr(&new), &mut name);
    if !dp.is_null() {
      ilock(dp);
      // dp may have been unlinked while we resolved it; linking into an
      // orphaned directory leaks ip (itrunc discards the record without
      // dropping ip->nlink).  create() has the same guard.
      if (*dp).nlink == 0 || (*dp).dev != (*ip).dev || dirlink(dp, &name, (*ip).inum) < 0 {
        iunlockput(dp);
      } else {
        iunlockput(dp);
        iput(ip);
        end_op();
        return 0;
      }
    }

    // bad:
    ilock(ip);
    (*ip).nlink -= 1;
    iupdate(ip);
    iunlockput(ip);
    end_op();
  }
  u64::MAX
}

// Is the directory dp empty except for "." and ".." ?
fn isdirempty(dp: *mut Inode) -> bool {
  let mut de = Dirent::new();
  let sz = size_of::<Dirent>() as u32;

  let mut off = 2 * sz;
  unsafe {
    while off < (*dp).size {
      if readi(dp, false, &raw mut de as u64, off, sz) != sz as i32 {
        panic("isdirempty: readi");
      }
      if de.inum != 0 {
        return false;
      }
      off += sz;
    }
  }
  true
}

pub fn sys_unlink() -> u64 {
  let mut name = [0u8; DIRSIZ];
  let mut path = [0u8; MAXPATH];
  let mut off: u32 = 0;

  if argstr(0, &mut path) < 0 {
    return u64::MAX;
  }

  begin_op();
  let dp = nameiparent(cstr(&path), &mut name);
  if dp.is_null() {
    end_op();
    return u64::MAX;
  }

  ilock(dp);

  // Cannot unlink "." or "..".
  if namecmp(&name, b".") == 0 || namecmp(&name, b"..") == 0 {
    iunlockput(dp);
    end_op();
    return u64::MAX;
  }

  let ip = dirlookup(dp, &name, Some(&mut off));
  if ip.is_null() {
    iunlockput(dp);
    end_op();
    return u64::MAX;
  }
  ilock(ip);

  unsafe {
    if (*ip).nlink < 1 {
      panic("unlink: nlink < 1");
    }
    if (*ip).typ == T_DIR && !isdirempty(ip) {
      iunlockput(ip);
      iunlockput(dp);
      end_op();
      return u64::MAX;
    }

    let de = Dirent::new();
    let sz = size_of::<Dirent>() as u32;
    if writei(dp, false, &raw const de as u64, off, sz) != sz as i32 {
      panic("unlink: writei");
    }
    if (*ip).typ == T_DIR {
      (*dp).nlink -= 1;
      iupdate(dp);
    }
    iunlockput(dp);

    (*ip).nlink -= 1;
    iupdate(ip);
    iunlockput(ip);
  }

  end_op();

  0
}

fn create(path: &[u8], typ: i16, major: i16, minor: i16) -> *mut Inode {
  let mut name = [0u8; DIRSIZ];

  let dp = nameiparent(path, &mut name);
  if dp.is_null() {
    return null_mut();
  }

  ilock(dp);

  unsafe {
    // the parent directory has been removed?
    if (*dp).nlink == 0 {
      iunlockput(dp);
      return null_mut();
    }

    let ip = dirlookup(dp, &name, None);
    if !ip.is_null() {
      iunlockput(dp);
      ilock(ip);
      if typ == T_FILE && ((*ip).typ == T_FILE || (*ip).typ == T_DEVICE) {
        return ip;
      }
      iunlockput(ip);
      return null_mut();
    }

    // a new directory's ".." would push dp->nlink past its maximum
    if typ == T_DIR && (*dp).nlink >= NLINK_MAX {
      iunlockput(dp);
      return null_mut();
    }

    let ip = ialloc((*dp).dev, typ);
    if ip.is_null() {
      iunlockput(dp);
      return null_mut();
    }

    ilock(ip);
    (*ip).major = major;
    (*ip).minor = minor;
    (*ip).nlink = 1;
    iupdate(ip);

    let ok = 'ok: {
      if typ == T_DIR {
        // Create . and .. entries.
        // No ip->nlink++ for ".": avoid cyclic ref count.
        if dirlink(ip, b".", (*ip).inum) < 0 || dirlink(ip, b"..", (*dp).inum) < 0 {
          break 'ok false;
        }
      }

      if dirlink(dp, &name, (*ip).inum) < 0 {
        break 'ok false;
      }
      true
    };

    if !ok {
      // something went wrong. de-allocate ip.
      (*ip).nlink = 0;
      iupdate(ip);
      iunlockput(ip);
      iunlockput(dp);
      return null_mut();
    }

    if typ == T_DIR {
      // now that success is guaranteed:
      (*dp).nlink += 1; // for ".."
      iupdate(dp);
    }

    iunlockput(dp);

    ip
  }
}

pub fn sys_open() -> u64 {
  let mut path = [0u8; MAXPATH];
  let mut omode: i32 = 0;

  argint(1, &mut omode);
  if argstr(0, &mut path) < 0 {
    return u64::MAX;
  }

  begin_op();

  let ip;
  if omode & O_CREATE != 0 {
    ip = create(cstr(&path), T_FILE, 0, 0);
    if ip.is_null() {
      end_op();
      return u64::MAX;
    }
  } else {
    ip = namei(cstr(&path));
    if ip.is_null() {
      end_op();
      return u64::MAX;
    }
    ilock(ip);
    unsafe {
      if (*ip).typ == T_DIR && omode != O_RDONLY {
        iunlockput(ip);
        end_op();
        return u64::MAX;
      }
    }
  }

  unsafe {
    if (*ip).typ == T_DEVICE && ((*ip).major < 0 || (*ip).major as usize >= NDEV) {
      iunlockput(ip);
      end_op();
      return u64::MAX;
    }

    let f = filealloc();
    let fd = if f.is_null() { -1 } else { fdalloc(f) };
    if fd < 0 {
      if !f.is_null() {
        fileclose(f);
      }
      iunlockput(ip);
      end_op();
      return u64::MAX;
    }

    if (*ip).typ == T_DEVICE {
      (*f).typ = FileType::FD_DEVICE;
      (*f).major = (*ip).major;
    } else {
      (*f).typ = FileType::FD_INODE;
      (*f).off = 0;
    }
    (*f).ip = ip;
    (*f).readable = omode & O_WRONLY == 0;
    (*f).writable = (omode & O_WRONLY) != 0 || (omode & O_RDWR) != 0;

    if (omode & O_TRUNC) != 0 && (*ip).typ == T_FILE {
      itrunc(ip);
    }

    iunlock(ip);
    end_op();

    fd as u64
  }
}

pub fn sys_mkdir() -> u64 {
  let mut path = [0u8; MAXPATH];

  begin_op();
  if argstr(0, &mut path) < 0 {
    end_op();
    return u64::MAX;
  }
  let ip = create(cstr(&path), T_DIR, 0, 0);
  if ip.is_null() {
    end_op();
    return u64::MAX;
  }
  iunlockput(ip);
  end_op();
  0
}

pub fn sys_mknod() -> u64 {
  let mut path = [0u8; MAXPATH];
  let mut major: i32 = 0;
  let mut minor: i32 = 0;

  begin_op();
  argint(1, &mut major);
  argint(2, &mut minor);
  if argstr(0, &mut path) < 0 {
    end_op();
    return u64::MAX;
  }
  let ip = create(cstr(&path), T_DEVICE, major as i16, minor as i16);
  if ip.is_null() {
    end_op();
    return u64::MAX;
  }
  iunlockput(ip);
  end_op();
  0
}

pub fn sys_chdir() -> u64 {
  let mut path = [0u8; MAXPATH];
  let p = myproc();

  begin_op();
  if argstr(0, &mut path) < 0 {
    end_op();
    return u64::MAX;
  }
  let ip = namei(cstr(&path));
  if ip.is_null() {
    end_op();
    return u64::MAX;
  }
  ilock(ip);
  unsafe {
    if (*ip).typ != T_DIR {
      iunlockput(ip);
      end_op();
      return u64::MAX;
    }
    iunlock(ip);
    iput((*p).cwd);
    end_op();
    (*p).cwd = ip;
  }
  0
}

pub fn sys_exec() -> u64 {
  let mut path = [0u8; MAXPATH];
  let mut argv: [*mut u8; MAXARG] = [null_mut(); MAXARG];
  let mut uargv: u64 = 0;

  argaddr(1, &mut uargv);
  if argstr(0, &mut path) < 0 {
    return u64::MAX;
  }

  let free_argv = |argv: &[*mut u8; MAXARG]| {
    for &a in argv.iter() {
      if a.is_null() {
        break;
      }
      kfree(a);
    }
  };

  let mut i = 0;
  loop {
    if i >= MAXARG {
      free_argv(&argv);
      return u64::MAX;
    }
    let mut uarg: u64 = 0;
    if fetchaddr(uargv + (size_of::<u64>() * i) as u64, &mut uarg) < 0 {
      free_argv(&argv);
      return u64::MAX;
    }
    if uarg == 0 {
      argv[i] = null_mut();
      break;
    }
    argv[i] = kalloc();
    if argv[i].is_null() {
      free_argv(&argv);
      return u64::MAX;
    }
    let buf = unsafe { core::slice::from_raw_parts_mut(argv[i], PGSIZE as usize) };
    if fetchstr(uarg, buf) < 0 {
      free_argv(&argv);
      return u64::MAX;
    }
    i += 1;
  }

  let mut kargv: [*const u8; MAXARG] = [null(); MAXARG];
  for j in 0..MAXARG {
    kargv[j] = argv[j];
  }
  let ret = kexec(cstr(&path), &kargv);

  free_argv(&argv);

  ret as u64
}

pub fn sys_pipe() -> u64 {
  let mut fdarray: u64 = 0; // user pointer to array of two integers
  let mut rf: *mut File = null_mut();
  let mut wf: *mut File = null_mut();
  let p = myproc();

  argaddr(0, &mut fdarray);
  if pipealloc(&mut rf, &mut wf) < 0 {
    return u64::MAX;
  }
  let fd0 = fdalloc(rf);
  let fd1 = if fd0 < 0 { -1 } else { fdalloc(wf) };
  unsafe {
    if fd0 < 0 || fd1 < 0 {
      if fd0 >= 0 {
        (*p).ofile[fd0 as usize] = null_mut();
      }
      fileclose(rf);
      fileclose(wf);
      return u64::MAX;
    }
    let isz = size_of::<i32>() as u64;
    if copyout((*p).pagetable, fdarray, &raw const fd0 as *const u8, isz) < 0
      || copyout((*p).pagetable, fdarray + isz, &raw const fd1 as *const u8, isz) < 0
    {
      (*p).ofile[fd0 as usize] = null_mut();
      (*p).ofile[fd1 as usize] = null_mut();
      fileclose(rf);
      fileclose(wf);
      return u64::MAX;
    }
  }
  0
}
