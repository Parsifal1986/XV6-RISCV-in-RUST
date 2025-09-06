use std::path;

use crate::{file::Inode, spinlock::{initlock, Spinlock}, stat::Stat, types};

pub const ROOTINO: u32 = 1;
pub const BSIZE: usize = 1024;

#[repr(C)]
pub struct Superblock {
  magic: u32,
  size: u32,
  nblocks: u32,
  ninodes: u32,
  nlog: u32,
  logstart: u32,
  inodestart: u32,
  bmapstart: u32,
}

impl Superblock {
  pub const fn new() -> Self {
    Superblock {
      magic: FSMAGIC,
      size: 0,
      nblocks: 0,
      ninodes: 0,
      nlog: 0,
      logstart: 0,
      inodestart: 0,
      bmapstart: 0,
    }
  }
}

struct Itable {
  lock: Spinlock,
  inode: [Inode; NDIRECT],
}

impl Itable {
  pub const fn new() -> Self {
    Itable {
      lock: Spinlock::new(),
      inode: [const { Inode::new() }; NDIRECT],
    }
  }
}

static mut ITABLE: Itable = Itable::new();

pub const FSMAGIC: u32 = 0x10203040;

pub const NDIRECT: usize = 12;
pub const NINDIRECT: usize = BSIZE / std::mem::size_of::<u32>();
pub const MAXFILE: usize = NDIRECT + NINDIRECT;

#[repr(C)]
pub struct Dinode {
  inode_type: i16,
  major: i16,
  minor: i16,
  nlink: i16,
  size: u32,
  addrs: [u32; NDIRECT + 1],
}

pub const IPB: usize = BSIZE / std::mem::size_of::<Dinode>();

fn iblock(i: u32, sb: &Superblock) -> u32 {
  (i / IPB as u32) + sb.inodestart
}

pub const BPB: usize = BSIZE * 8;

fn bblock(b: u32, sb: &Superblock) -> u32 {
  b / BPB as u32 + sb.bmapstart
}

pub const DIRSIZ: usize = 14;

#[repr(C)]
pub struct Dirent {
  inum: u16,
  name: [u8; DIRSIZ],
}

static mut SB: Superblock = Superblock::new();

pub fn readsb(dev: i32, sb: &mut Superblock) {
  todo!()
}

pub fn fsinit(dev: i32) {
  todo!()
}

pub fn bzero(dev: i32, bno: i32) {
  todo!()
}

pub fn balloc(dev: u32) -> u32 {
  todo!()
}

pub fn bfree(dev: i32, b: u32) {
  todo!()
}

pub fn iinit() {
  let i = 0;
  todo!()
}

pub fn ialloc(dev: u32, inode_type: i16) -> Option<*mut Inode> {
  todo!()
}

pub fn iupdate(ip: *mut Inode) {
  todo!()
}

fn iget(dev: u32, inum: u32) -> Option<*mut Inode> {
  todo!()
}

pub fn idup(ip: *mut Inode) -> *mut Inode {
  todo!()
}

pub fn ilock(ip: Option<*mut Inode>) {

}

pub fn iunlock(ip: Option<*mut Inode>) {
  todo!()
}

pub fn iput(ip: *mut Inode) {
  todo!()
}

pub fn iunlockput(ip: *mut Inode) {
  todo!()
}

pub fn ireclaim(dev: i32) {
  todo!()
}

pub fn bmap(ip: *mut Inode, bn: u32) -> u32 {
  todo!()
}

pub fn itrunc(ip: *mut Inode) {
  todo!()
}

pub fn stati(ip: *mut Inode, st: &mut Stat) {
  todo!()
}

pub fn readi(inode: *mut Inode, user_dst: i32, dst: u64, off: u64, n: u32) -> i32 {
  todo!()
}

pub fn writei(inode: *mut Inode, src: u64, off: u64, n: u32) -> i32 {
  todo!()
}

pub fn namescmp(s: &[u8], t: &[u8]) -> i32 {
  todo!()
}

pub fn dirlink(dp: *mut Inode, name: &[u8], inum: u32) -> i32 {
  todo!()
}

pub fn skipelem(path: &[u8], name: &[u8]) -> &'static str {
  todo!()
}

pub fn namex(path: &[u8], nameiparent: i32, name: &[u8]) -> Option<*mut Inode> {
  todo!()
}

pub fn namei(path: &[u8]) -> Option<*mut Inode> {
  todo!()
}

pub fn nameiparent(path: &[u8], name: &[u8]) -> Option<*mut Inode> {
  todo!()
}