// File system implementation.  Five layers:
//   + Blocks: allocator for raw disk blocks.
//   + Log: crash recovery for multi-step updates.
//   + Files: inode allocator, reading, writing, metadata.
//   + Directories: inode with special contents (list of other inodes!)
//   + Names: paths like /usr/rtm/xv6/fs.c for convenient naming.
//
// This file contains the low-level file system manipulation
// routines.  The (higher-level) system call implementations
// are in sysfile.rs.

use core::cmp::min;
use core::ptr::{copy, null_mut, write_bytes};

use crate::bio::{bread, brelse};
use crate::file::Inode;
use crate::log::{begin_op, end_op, initlog, log_write};
use crate::param::{NINODE, ROOTDEV};
use crate::printf::panic;
use crate::proc::{either_copyin, either_copyout, myproc};
use crate::sleeplock::{acquiresleep, holdingsleep, initsleeplock, releasesleep};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::stat::{Stat, T_DIR};
use crate::string::strncmp;

// On-disk file system format.
// Both the kernel and user programs use this header file.

pub const ROOTINO: u32 = 1; // root i-number
pub const BSIZE: usize = 1024; // block size

// Disk layout:
// [ boot block | super block | log | inode blocks |
//                                          free bit map | data blocks]
//
// mkfs computes the super block and builds an initial file system. The
// super block describes the disk layout:
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Superblock {
  pub magic: u32,      // Must be FSMAGIC
  pub size: u32,       // Size of file system image (blocks)
  pub nblocks: u32,    // Number of data blocks
  pub ninodes: u32,    // Number of inodes.
  pub nlog: u32,       // Number of log blocks
  pub logstart: u32,   // Block number of first log block
  pub inodestart: u32, // Block number of first inode block
  pub bmapstart: u32,  // Block number of first free map block
}

pub const FSMAGIC: u32 = 0x10203040;

pub const NDIRECT: usize = 12;
pub const NINDIRECT: usize = BSIZE / size_of::<u32>();
pub const MAXFILE: usize = NDIRECT + NINDIRECT;
pub const NLINK_MAX: i16 = i16::MAX; // nlink is a short; refuse links past its maximum

// On-disk inode structure
#[repr(C)]
pub struct Dinode {
  pub typ: i16,                   // File type
  pub major: i16,                 // Major device number (T_DEVICE only)
  pub minor: i16,                 // Minor device number (T_DEVICE only)
  pub nlink: i16,                 // Number of links to inode in file system
  pub size: u32,                  // Size of file (bytes)
  pub addrs: [u32; NDIRECT + 1],  // Data block addresses
}

// Inodes per block.
pub const IPB: usize = BSIZE / size_of::<Dinode>();

// Block containing inode i
#[inline(always)]
pub fn IBLOCK(i: u32, sb: &Superblock) -> u32 {
  i / IPB as u32 + sb.inodestart
}

// Bitmap bits per block
pub const BPB: usize = BSIZE * 8;

// Block of free map containing bit for block b
#[inline(always)]
pub fn BBLOCK(b: u32, sb: &Superblock) -> u32 {
  b / BPB as u32 + sb.bmapstart
}

// Directory is a file containing a sequence of dirent structures.
pub const DIRSIZ: usize = 14;

// The name field may have DIRSIZ characters and not end in a NUL
// character.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dirent {
  pub inum: u16,
  pub name: [u8; DIRSIZ],
}

impl Dirent {
  pub const fn new() -> Self {
    Dirent { inum: 0, name: [0; DIRSIZ] }
  }
}

// there should be one superblock per disk device, but we run with
// only one device
static mut SB: Superblock = Superblock {
  magic: 0,
  size: 0,
  nblocks: 0,
  ninodes: 0,
  nlog: 0,
  logstart: 0,
  inodestart: 0,
  bmapstart: 0,
};

fn sb() -> &'static Superblock {
  unsafe { &*(&raw const SB) }
}

// the address of the i'th dinode in a buffer holding an inode block.
fn dinode_at(data: *mut u8, inum: u32) -> *mut Dinode {
  unsafe { (data as *mut Dinode).add(inum as usize % IPB) }
}

// Read the super block.
fn readsb(dev: u32, sb: *mut Superblock) {
  let bp = bread(dev, 1);
  unsafe {
    copy((*bp).data.as_ptr(), sb as *mut u8, size_of::<Superblock>());
  }
  brelse(bp);
}

// Init fs
pub fn fsinit(dev: u32) {
  readsb(dev, &raw mut SB);
  if sb().magic != FSMAGIC {
    panic("invalid file system");
  }
  initlog(dev, sb());
  ireclaim(dev);
}

// Zero a block.
fn bzero(dev: u32, bno: u32) {
  let bp = bread(dev, bno);
  unsafe {
    write_bytes((*bp).data.as_mut_ptr(), 0, BSIZE);
  }
  log_write(bp);
  brelse(bp);
}

// Blocks.

// Allocate a zeroed disk block.
// returns 0 if out of disk space.
fn balloc(dev: u32) -> u32 {
  let size = sb().size;
  let mut b: u32 = 0;
  while b < size {
    let bp = bread(dev, BBLOCK(b, sb()));
    let mut bi: u32 = 0;
    while bi < BPB as u32 && b + bi < size {
      let m = 1u8 << (bi % 8);
      unsafe {
        if (*bp).data[bi as usize / 8] & m == 0 {
          // Is block free?
          (*bp).data[bi as usize / 8] |= m; // Mark block in use.
          log_write(bp);
          brelse(bp);
          bzero(dev, b + bi);
          return b + bi;
        }
      }
      bi += 1;
    }
    brelse(bp);
    b += BPB as u32;
  }
  printf!("balloc: out of blocks\n");
  0
}

// Free a disk block.
fn bfree(dev: u32, b: u32) {
  let bp = bread(dev, BBLOCK(b, sb()));
  let bi = b as usize % BPB;
  let m = 1u8 << (bi % 8);
  unsafe {
    if (*bp).data[bi / 8] & m == 0 {
      panic("freeing free block");
    }
    (*bp).data[bi / 8] &= !m;
  }
  log_write(bp);
  brelse(bp);
}

// Inodes.
//
// An inode describes a single unnamed file.
// The inode disk structure holds metadata: the file's type,
// its size, the number of links referring to it, and the
// list of blocks holding the file's content.
//
// The inodes are laid out sequentially on disk at block
// sb.inodestart. Each inode has a number, indicating its
// position on the disk.
//
// The kernel keeps a table of in-use inodes in memory
// to provide a place for synchronizing access
// to inodes used by multiple processes. The in-memory
// inodes include book-keeping information that is
// not stored on disk: ip->ref and ip->valid.
//
// An inode and its in-memory representation go through a
// sequence of states before they can be used by the
// rest of the file system code.
//
// * Allocation: an inode is allocated if its type (on disk)
//   is non-zero. ialloc() allocates, and iput() frees if
//   the reference and link counts have fallen to zero.
//
// * Referencing in table: an entry in the inode table
//   is free if ip->ref is zero. Otherwise ip->ref tracks
//   the number of in-memory pointers to the entry (open
//   files and current directories). iget() finds or
//   creates a table entry and increments its ref; iput()
//   decrements ref.
//
// * Valid: the information (type, size, &c) in an inode
//   table entry is only correct when ip->valid is 1.
//   ilock() reads the inode from
//   the disk and sets ip->valid, while iput() clears
//   ip->valid if ip->ref has fallen to zero.
//
// * Locked: file system code may only examine and modify
//   the information in an inode and its content if it
//   has first locked the inode.
//
// Thus a typical sequence is:
//   ip = iget(dev, inum)
//   ilock(ip)
//   ... examine and modify ip->xxx ...
//   iunlock(ip)
//   iput(ip)
//
// ilock() is separate from iget() so that system calls can
// get a long-term reference to an inode (as for an open file)
// and only lock it for short periods (e.g., in read()).
// The separation also helps avoid deadlock and races during
// pathname lookup. iget() increments ip->ref so that the inode
// stays in the table and pointers to it remain valid.
//
// Many internal file system functions expect the caller to
// have locked the inodes involved; this lets callers create
// multi-step atomic operations.
//
// The ITABLE.lock spin-lock protects the allocation of itable
// entries. Since ip->ref indicates whether an entry is free,
// and ip->dev and ip->inum indicate which i-node an entry
// holds, one must hold ITABLE.lock while using any of those fields.
//
// An ip->lock sleep-lock protects all ip-> fields other than ref,
// dev, and inum.  One must hold ip->lock in order to
// read or write that inode's ip->valid, ip->size, ip->type, &c.

struct Itable {
  lock: Spinlock,
  inode: [Inode; NINODE],
}

static mut ITABLE: Itable = Itable {
  lock: Spinlock::new(),
  inode: [const { Inode::new() }; NINODE],
};

pub fn iinit() {
  unsafe {
    initlock(&raw mut ITABLE.lock, "itable");
    for i in 0..NINODE {
      initsleeplock(&raw mut ITABLE.inode[i].lock, "inode");
    }
  }
}

// Allocate an inode on device dev.
// Mark it as allocated by  giving it type typ.
// Returns an unlocked but allocated and referenced inode,
// or null if there is no free inode.
pub fn ialloc(dev: u32, typ: i16) -> *mut Inode {
  for inum in 1..sb().ninodes {
    let bp = bread(dev, IBLOCK(inum, sb()));
    unsafe {
      let dip = dinode_at((*bp).data.as_mut_ptr(), inum);
      if (*dip).typ == 0 {
        // a free inode
        write_bytes(dip as *mut u8, 0, size_of::<Dinode>());
        (*dip).typ = typ;
        log_write(bp); // mark it allocated on the disk
        brelse(bp);
        return iget(dev, inum);
      }
    }
    brelse(bp);
  }
  printf!("ialloc: no inodes\n");
  null_mut()
}

// Copy a modified in-memory inode to disk.
// Must be called after every change to an ip->xxx field
// that lives on disk.
// Caller must hold ip->lock.
pub fn iupdate(ip: *mut Inode) {
  unsafe {
    let bp = bread((*ip).dev, IBLOCK((*ip).inum, sb()));
    let dip = dinode_at((*bp).data.as_mut_ptr(), (*ip).inum);
    (*dip).typ = (*ip).typ;
    (*dip).major = (*ip).major;
    (*dip).minor = (*ip).minor;
    (*dip).nlink = (*ip).nlink;
    (*dip).size = (*ip).size;
    (*dip).addrs = (*ip).addrs;
    log_write(bp);
    brelse(bp);
  }
}

// Find the inode with number inum on device dev
// and return the in-memory copy. Does not lock
// the inode and does not read it from disk.
fn iget(dev: u32, inum: u32) -> *mut Inode {
  unsafe {
    acquire(&raw mut ITABLE.lock);

    // Is the inode already in the table?
    let mut empty: *mut Inode = null_mut();
    for i in 0..NINODE {
      let ip = &raw mut ITABLE.inode[i];
      if (*ip).r#ref > 0 && (*ip).dev == dev && (*ip).inum == inum {
        (*ip).r#ref += 1;
        release(&raw mut ITABLE.lock);
        return ip;
      }
      if empty.is_null() && (*ip).r#ref == 0 {
        // Remember empty slot.
        empty = ip;
      }
    }

    // Recycle an inode entry.
    if empty.is_null() {
      panic("iget: no inodes");
    }

    let ip = empty;
    (*ip).dev = dev;
    (*ip).inum = inum;
    (*ip).r#ref = 1;
    (*ip).valid = false;
    release(&raw mut ITABLE.lock);

    ip
  }
}

// Increment reference count for ip.
// Returns ip to enable ip = idup(ip1) idiom.
pub fn idup(ip: *mut Inode) -> *mut Inode {
  unsafe {
    acquire(&raw mut ITABLE.lock);
    (*ip).r#ref += 1;
    release(&raw mut ITABLE.lock);
  }
  ip
}

// Lock the given inode.
// Reads the inode from disk if necessary.
pub fn ilock(ip: *mut Inode) {
  unsafe {
    if ip.is_null() || (*ip).r#ref < 1 {
      panic("ilock");
    }

    acquiresleep(&raw mut (*ip).lock);

    if !(*ip).valid {
      let bp = bread((*ip).dev, IBLOCK((*ip).inum, sb()));
      let dip = dinode_at((*bp).data.as_mut_ptr(), (*ip).inum);
      (*ip).typ = (*dip).typ;
      (*ip).major = (*dip).major;
      (*ip).minor = (*dip).minor;
      (*ip).nlink = (*dip).nlink;
      (*ip).size = (*dip).size;
      (*ip).addrs = (*dip).addrs;
      brelse(bp);
      (*ip).valid = true;
      if (*ip).typ == 0 {
        panic("ilock: no type");
      }
    }
  }
}

// Unlock the given inode.
pub fn iunlock(ip: *mut Inode) {
  unsafe {
    if ip.is_null() || !holdingsleep(&raw mut (*ip).lock) || (*ip).r#ref < 1 {
      panic("iunlock");
    }

    releasesleep(&raw mut (*ip).lock);
  }
}

// Mark the on-disk inode free.
fn ifree(dev: u32, inum: u32) {
  let bp = bread(dev, IBLOCK(inum, sb()));
  unsafe {
    let dip = dinode_at((*bp).data.as_mut_ptr(), inum);
    (*dip).typ = 0;
  }
  log_write(bp);
  brelse(bp);
}

// Drop a reference to an in-memory inode.
// If that was the last reference, the inode table entry can
// be recycled.
// If that was the last reference and the inode has no links
// to it, free the inode (and its content) on disk.
// All calls to iput() must be inside a transaction in
// case it has to free the inode.
pub fn iput(ip: *mut Inode) {
  unsafe {
    acquire(&raw mut ITABLE.lock);

    // Last reference of an unlinked inode?  Capture dev/inum before ref--,
    // since once ref hits 0, ip may be recycled by a concurrent iget()
    // for a different inum.
    let last = (*ip).r#ref == 1 && (*ip).valid && (*ip).nlink == 0;
    let dev = (*ip).dev;
    let inum = (*ip).inum;

    if last {
      // ip->ref == 1 means no other process can have ip locked,
      // so this acquiresleep() won't block (or deadlock).
      acquiresleep(&raw mut (*ip).lock);

      release(&raw mut ITABLE.lock);

      itrunc(ip); // free the data blocks (type stays nonzero on disk)
      (*ip).valid = false;

      releasesleep(&raw mut (*ip).lock);

      acquire(&raw mut ITABLE.lock);
    }

    (*ip).r#ref -= 1;
    release(&raw mut ITABLE.lock);

    if last {
      ifree(dev, inum); // now clear type on disk: inum becomes allocatable
    }
  }
}

// Common idiom: unlock, then put.
pub fn iunlockput(ip: *mut Inode) {
  iunlock(ip);
  iput(ip);
}

// Free inodes that have no links but that were still open
// when the system crashed or was shut down.
pub fn ireclaim(dev: u32) {
  for inum in 1..sb().ninodes {
    let mut ip: *mut Inode = null_mut();
    let bp = bread(dev, IBLOCK(inum, sb()));
    unsafe {
      let dip = dinode_at((*bp).data.as_mut_ptr(), inum);
      if (*dip).typ != 0 && (*dip).nlink == 0 {
        // is an orphaned inode
        printf!("ireclaim: orphaned inode {}\n", inum);
        ip = iget(dev, inum);
      }
    }
    brelse(bp);
    if !ip.is_null() {
      begin_op();
      ilock(ip);
      iunlock(ip);
      iput(ip);
      end_op();
    }
  }
}

// Inode content
//
// The content (data) associated with each inode is stored
// in blocks on the disk. The first NDIRECT block numbers
// are listed in ip->addrs[].  The next NINDIRECT blocks are
// listed in block ip->addrs[NDIRECT].

// Return the disk block address of the nth block in inode ip.
// If there is no such block, bmap allocates one.
// returns 0 if out of disk space.
fn bmap(ip: *mut Inode, bn: u32) -> u32 {
  unsafe {
    let mut bn = bn as usize;
    if bn < NDIRECT {
      let mut addr = (*ip).addrs[bn];
      if addr == 0 {
        addr = balloc((*ip).dev);
        if addr == 0 {
          return 0;
        }
        (*ip).addrs[bn] = addr;
      }
      return addr;
    }
    bn -= NDIRECT;

    if bn < NINDIRECT {
      // Load indirect block, allocating if necessary.
      let mut addr = (*ip).addrs[NDIRECT];
      if addr == 0 {
        addr = balloc((*ip).dev);
        if addr == 0 {
          return 0;
        }
        (*ip).addrs[NDIRECT] = addr;
      }
      let bp = bread((*ip).dev, addr);
      let a = (*bp).data.as_mut_ptr() as *mut u32;
      addr = *a.add(bn);
      if addr == 0 {
        addr = balloc((*ip).dev);
        if addr != 0 {
          *a.add(bn) = addr;
          log_write(bp);
        }
      }
      brelse(bp);
      return addr;
    }
  }

  panic("bmap: out of range");
}

// Truncate inode (discard contents).
// Caller must hold ip->lock.
pub fn itrunc(ip: *mut Inode) {
  unsafe {
    for i in 0..NDIRECT {
      if (*ip).addrs[i] != 0 {
        bfree((*ip).dev, (*ip).addrs[i]);
        (*ip).addrs[i] = 0;
      }
    }

    if (*ip).addrs[NDIRECT] != 0 {
      let bp = bread((*ip).dev, (*ip).addrs[NDIRECT]);
      let a = (*bp).data.as_ptr() as *const u32;
      for j in 0..NINDIRECT {
        if *a.add(j) != 0 {
          bfree((*ip).dev, *a.add(j));
        }
      }
      brelse(bp);
      bfree((*ip).dev, (*ip).addrs[NDIRECT]);
      (*ip).addrs[NDIRECT] = 0;
    }

    (*ip).size = 0;
  }
  iupdate(ip);
}

// Copy stat information from inode.
// Caller must hold ip->lock.
pub fn stati(ip: *mut Inode, st: &mut Stat) {
  unsafe {
    st.dev = (*ip).dev as i32;
    st.ino = (*ip).inum;
    st.typ = (*ip).typ;
    st.nlink = (*ip).nlink;
    st.size = (*ip).size as u64;
  }
}

// Read data from inode.
// Caller must hold ip->lock.
// If user_dst is true, then dst is a user virtual address;
// otherwise, dst is a kernel address.
pub fn readi(ip: *mut Inode, user_dst: bool, mut dst: u64, mut off: u32, mut n: u32) -> i32 {
  unsafe {
    if off > (*ip).size || off.wrapping_add(n) < off {
      return 0;
    }
    if off + n > (*ip).size {
      n = (*ip).size - off;
    }

    let mut tot: u32 = 0;
    while tot < n {
      let addr = bmap(ip, off / BSIZE as u32);
      if addr == 0 {
        break;
      }
      let bp = bread((*ip).dev, addr);
      let m = min(n - tot, BSIZE as u32 - off % BSIZE as u32);
      let src = (*bp).data.as_ptr().add(off as usize % BSIZE);
      if either_copyout(user_dst, dst, src, m as u64) == -1 {
        brelse(bp);
        return -1;
      }
      brelse(bp);
      tot += m;
      off += m;
      dst += m as u64;
    }
    tot as i32
  }
}

// Write data to inode.
// Caller must hold ip->lock.
// If user_src is true, then src is a user virtual address;
// otherwise, src is a kernel address.
// Returns the number of bytes successfully written.
// If the return value is less than the requested n,
// there was an error of some kind.
pub fn writei(ip: *mut Inode, user_src: bool, mut src: u64, mut off: u32, n: u32) -> i32 {
  unsafe {
    if off > (*ip).size || off.wrapping_add(n) < off {
      return -1;
    }
    if off as u64 + n as u64 > (MAXFILE * BSIZE) as u64 {
      return -1;
    }

    let mut tot: u32 = 0;
    while tot < n {
      let addr = bmap(ip, off / BSIZE as u32);
      if addr == 0 {
        break;
      }
      let bp = bread((*ip).dev, addr);
      let m = min(n - tot, BSIZE as u32 - off % BSIZE as u32);
      let dst = (*bp).data.as_mut_ptr().add(off as usize % BSIZE);
      if either_copyin(dst, user_src, src, m as u64) == -1 {
        // Might have partially updated the block, so we need to log it.
        log_write(bp);
        brelse(bp);
        break;
      }
      log_write(bp);
      brelse(bp);
      tot += m;
      off += m;
      src += m as u64;
    }

    if off > (*ip).size {
      (*ip).size = off;
    }

    // write the i-node back to disk even if the size didn't change
    // because the loop above might have called bmap() and added a new
    // block to ip->addrs[].
    iupdate(ip);

    tot as i32
  }
}

// Directories

pub fn namecmp(s: &[u8], t: &[u8]) -> i32 {
  strncmp(s, t, DIRSIZ)
}

// Look for a directory entry in a directory.
// If found, set *poff to byte offset of entry.
pub fn dirlookup(dp: *mut Inode, name: &[u8], poff: Option<&mut u32>) -> *mut Inode {
  let mut de = Dirent::new();
  let sz = size_of::<Dirent>() as u32;

  unsafe {
    if (*dp).typ != T_DIR {
      panic("dirlookup not DIR");
    }

    let mut off = 0;
    while off < (*dp).size {
      if readi(dp, false, &raw mut de as u64, off, sz) != sz as i32 {
        panic("dirlookup read");
      }
      if de.inum != 0 && namecmp(name, &de.name) == 0 {
        // entry matches path element
        if let Some(poff) = poff {
          *poff = off;
        }
        return iget((*dp).dev, de.inum as u32);
      }
      off += sz;
    }
  }

  null_mut()
}

// Write a new directory entry (name, inum) into the directory dp.
// Returns 0 on success, -1 on failure (e.g. out of disk blocks).
pub fn dirlink(dp: *mut Inode, name: &[u8], inum: u32) -> i32 {
  let mut de = Dirent::new();
  let sz = size_of::<Dirent>() as u32;

  // Check that name is not present.
  let ip = dirlookup(dp, name, None);
  if !ip.is_null() {
    iput(ip);
    return -1;
  }

  // Look for an empty dirent.
  let mut off = 0;
  unsafe {
    while off < (*dp).size {
      if readi(dp, false, &raw mut de as u64, off, sz) != sz as i32 {
        panic("dirlink read");
      }
      if de.inum == 0 {
        break;
      }
      off += sz;
    }
  }

  // strncpy(de.name, name, DIRSIZ)
  de.name = [0; DIRSIZ];
  for i in 0..DIRSIZ {
    if i >= name.len() || name[i] == 0 {
      break;
    }
    de.name[i] = name[i];
  }
  de.inum = inum as u16;
  if writei(dp, false, &raw const de as u64, off, sz) != sz as i32 {
    return -1;
  }

  0
}

// Paths

// Copy the next path element from path into name.
// Return the path following the copied element.
// The returned path has no leading slashes,
// so the caller can check whether it is empty to see if the name is the last one.
// If no name to remove, return None.
//
// Examples:
//   skipelem("a/bb/c", name) = "bb/c", setting name = "a"
//   skipelem("///a//bb", name) = "bb", setting name = "a"
//   skipelem("a", name) = "", setting name = "a"
//   skipelem("", name) = skipelem("////", name) = None
//
fn skipelem<'a>(path: &'a [u8], name: &mut [u8; DIRSIZ]) -> Option<&'a [u8]> {
  let at = |p: &[u8], i: usize| if i < p.len() { p[i] } else { 0 };

  let mut i = 0;
  while at(path, i) == b'/' {
    i += 1;
  }
  if at(path, i) == 0 {
    return None;
  }
  let s = i;
  while at(path, i) != b'/' && at(path, i) != 0 {
    i += 1;
  }
  let len = i - s;
  if len >= DIRSIZ {
    name.copy_from_slice(&path[s..s + DIRSIZ]);
  } else {
    name[..len].copy_from_slice(&path[s..i]);
    name[len] = 0;
  }
  while at(path, i) == b'/' {
    i += 1;
  }
  Some(&path[i..])
}

// Look up and return the inode for a path name.
// If parent is true, return the inode for the parent and copy the final
// path element into name, which must have room for DIRSIZ bytes.
// Must be called inside a transaction since it calls iput().
fn namex(mut path: &[u8], nameiparent: bool, name: &mut [u8; DIRSIZ]) -> *mut Inode {
  let mut ip = if !path.is_empty() && path[0] == b'/' {
    iget(ROOTDEV, ROOTINO)
  } else {
    unsafe { idup((*myproc()).cwd) }
  };

  while let Some(rest) = skipelem(path, name) {
    path = rest;
    ilock(ip);
    unsafe {
      if (*ip).typ != T_DIR {
        iunlockput(ip);
        return null_mut();
      }
      if (*ip).nlink == 0 {
        // the directory has been removed.
        iunlockput(ip);
        return null_mut();
      }
    }
    if nameiparent && (path.is_empty() || path[0] == 0) {
      // Stop one level early.
      iunlock(ip);
      return ip;
    }
    let next = dirlookup(ip, &name[..], None);
    if next.is_null() {
      iunlockput(ip);
      return null_mut();
    }
    iunlockput(ip);
    ip = next;
  }
  if nameiparent {
    iput(ip);
    return null_mut();
  }
  ip
}

pub fn namei(path: &[u8]) -> *mut Inode {
  let mut name = [0u8; DIRSIZ];
  namex(path, false, &mut name)
}

pub fn nameiparent(path: &[u8], name: &mut [u8; DIRSIZ]) -> *mut Inode {
  namex(path, true, name)
}
