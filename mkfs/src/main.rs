// mkfs: build an xv6 file system image.
//
//   mkfs fs.img files...
//
// Each file is copied into the root directory, named by
// its base name (with any leading '_' removed).

use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;

// These must agree with the kernel (kernel/src/param.rs, fs.rs, stat.rs).
const FSSIZE: u32 = 2000; // size of file system in blocks
const MAXOPBLOCKS: u32 = 10; // max # of blocks any FS op writes
const LOGBLOCKS: u32 = MAXOPBLOCKS * 3; // max data blocks in on-disk log

const ROOTINO: u32 = 1; // root i-number
const BSIZE: usize = 1024; // block size
const FSMAGIC: u32 = 0x10203040;
const NDIRECT: usize = 12;
const NINDIRECT: usize = BSIZE / 4;
const MAXFILE: usize = NDIRECT + NINDIRECT;
const DINODE_SIZE: usize = 64; // size of struct dinode
const IPB: u32 = (BSIZE / DINODE_SIZE) as u32; // inodes per block
const BPB: u32 = (BSIZE * 8) as u32; // bitmap bits per block
const DIRSIZ: usize = 14;
const DIRENT_SIZE: usize = 16; // size of struct dirent

const T_DIR: i16 = 1;
const T_FILE: i16 = 2;

const NINODES: u32 = 200;

// Disk layout:
// [ boot block | sb block | log | inode blocks | free bit map | data blocks ]

#[derive(Clone, Copy, Default)]
struct Superblock {
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
  fn encode(&self) -> Vec<u8> {
    [self.magic, self.size, self.nblocks, self.ninodes, self.nlog, self.logstart, self.inodestart, self.bmapstart]
      .iter()
      .flat_map(|x| x.to_le_bytes())
      .collect()
  }
}

// On-disk inode structure
#[derive(Clone, Copy, Default)]
struct Dinode {
  typ: i16,
  major: i16,
  minor: i16,
  nlink: i16,
  size: u32,
  addrs: [u32; NDIRECT + 1],
}

impl Dinode {
  fn encode(&self) -> [u8; DINODE_SIZE] {
    let mut b = [0u8; DINODE_SIZE];
    b[0..2].copy_from_slice(&self.typ.to_le_bytes());
    b[2..4].copy_from_slice(&self.major.to_le_bytes());
    b[4..6].copy_from_slice(&self.minor.to_le_bytes());
    b[6..8].copy_from_slice(&self.nlink.to_le_bytes());
    b[8..12].copy_from_slice(&self.size.to_le_bytes());
    for (i, a) in self.addrs.iter().enumerate() {
      b[12 + 4 * i..16 + 4 * i].copy_from_slice(&a.to_le_bytes());
    }
    b
  }

  fn decode(b: &[u8]) -> Dinode {
    let u16at = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
    let u32at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let mut addrs = [0u32; NDIRECT + 1];
    for (i, a) in addrs.iter_mut().enumerate() {
      *a = u32at(12 + 4 * i);
    }
    Dinode { typ: u16at(0), major: u16at(2), minor: u16at(4), nlink: u16at(6), size: u32at(8), addrs }
  }
}

fn dirent(inum: u32, name: &[u8]) -> [u8; DIRENT_SIZE] {
  let mut b = [0u8; DIRENT_SIZE];
  b[0..2].copy_from_slice(&(inum as u16).to_le_bytes());
  let n = name.len().min(DIRSIZ);
  b[2..2 + n].copy_from_slice(&name[..n]);
  b
}

struct Mkfs {
  img: Vec<u8>,
  sb: Superblock,
  freeinode: u32,
  freeblock: u32,
}

impl Mkfs {
  fn wsect(&mut self, sec: u32, buf: &[u8]) {
    assert!(buf.len() == BSIZE);
    let off = sec as usize * BSIZE;
    self.img[off..off + BSIZE].copy_from_slice(buf);
  }

  fn rsect(&self, sec: u32) -> Vec<u8> {
    let off = sec as usize * BSIZE;
    self.img[off..off + BSIZE].to_vec()
  }

  fn iblock(&self, inum: u32) -> u32 {
    inum / IPB + self.sb.inodestart
  }

  fn winode(&mut self, inum: u32, din: &Dinode) {
    let bn = self.iblock(inum);
    let mut buf = self.rsect(bn);
    let off = (inum % IPB) as usize * DINODE_SIZE;
    buf[off..off + DINODE_SIZE].copy_from_slice(&din.encode());
    self.wsect(bn, &buf);
  }

  fn rinode(&self, inum: u32) -> Dinode {
    let bn = self.iblock(inum);
    let buf = self.rsect(bn);
    let off = (inum % IPB) as usize * DINODE_SIZE;
    Dinode::decode(&buf[off..off + DINODE_SIZE])
  }

  fn ialloc(&mut self, typ: i16) -> u32 {
    let inum = self.freeinode;
    self.freeinode += 1;
    let din = Dinode { typ, nlink: 1, ..Default::default() };
    self.winode(inum, &din);
    inum
  }

  fn balloc(&mut self, used: u32) {
    println!("balloc: first {} blocks have been allocated", used);
    assert!(used < BPB);
    let mut buf = vec![0u8; BSIZE];
    for i in 0..used as usize {
      buf[i / 8] |= 1 << (i % 8);
    }
    println!("balloc: write bitmap block at sector {}", self.sb.bmapstart);
    let bmapstart = self.sb.bmapstart;
    self.wsect(bmapstart, &buf);
  }

  fn alloc_block(&mut self) -> u32 {
    let b = self.freeblock;
    self.freeblock += 1;
    if b >= FSSIZE {
      eprintln!("mkfs: file system image too small (FSSIZE = {})", FSSIZE);
      exit(1);
    }
    b
  }

  fn iappend(&mut self, inum: u32, mut p: &[u8]) {
    let mut din = self.rinode(inum);
    let mut off = din.size as usize;
    while !p.is_empty() {
      let fbn = off / BSIZE;
      if fbn >= MAXFILE {
        eprintln!("mkfs: file too large (max {} bytes)", MAXFILE * BSIZE);
        exit(1);
      }
      let x = if fbn < NDIRECT {
        if din.addrs[fbn] == 0 {
          din.addrs[fbn] = self.alloc_block();
        }
        din.addrs[fbn]
      } else {
        if din.addrs[NDIRECT] == 0 {
          din.addrs[NDIRECT] = self.alloc_block();
        }
        let ib = din.addrs[NDIRECT];
        let mut indirect = self.rsect(ib);
        let i = (fbn - NDIRECT) * 4;
        let mut a = u32::from_le_bytes([indirect[i], indirect[i + 1], indirect[i + 2], indirect[i + 3]]);
        if a == 0 {
          a = self.alloc_block();
          indirect[i..i + 4].copy_from_slice(&a.to_le_bytes());
          self.wsect(ib, &indirect);
        }
        a
      };
      let n1 = p.len().min((fbn + 1) * BSIZE - off);
      let mut buf = self.rsect(x);
      let boff = off - fbn * BSIZE;
      buf[boff..boff + n1].copy_from_slice(&p[..n1]);
      self.wsect(x, &buf);
      off += n1;
      p = &p[n1..];
    }
    din.size = off as u32;
    self.winode(inum, &din);
  }
}

fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    eprintln!("Usage: mkfs fs.img files...");
    exit(1);
  }

  assert!(BSIZE % DINODE_SIZE == 0);
  assert!(BSIZE % DIRENT_SIZE == 0);

  let nbitmap = FSSIZE / BPB + 1;
  let ninodeblocks = NINODES / IPB + 1;
  let nlog = LOGBLOCKS + 1; // Header followed by LOGBLOCKS data blocks.

  // 1 fs block = 1 disk sector
  let nmeta = 2 + nlog + ninodeblocks + nbitmap; // Number of meta blocks (boot, sb, nlog, inode, bitmap)
  let nblocks = FSSIZE - nmeta; // Number of data blocks

  let sb = Superblock {
    magic: FSMAGIC,
    size: FSSIZE,
    nblocks,
    ninodes: NINODES,
    nlog,
    logstart: 2,
    inodestart: 2 + nlog,
    bmapstart: 2 + nlog + ninodeblocks,
  };

  println!(
    "nmeta {} (boot, super, log blocks {}, inode blocks {}, bitmap blocks {}) blocks {} total {}",
    nmeta, nlog, ninodeblocks, nbitmap, nblocks, FSSIZE
  );

  let mut fs = Mkfs {
    img: vec![0u8; FSSIZE as usize * BSIZE],
    sb,
    freeinode: 1,
    freeblock: nmeta, // the first free block that we can allocate
  };

  let mut buf = vec![0u8; BSIZE];
  let sbb = sb.encode();
  buf[..sbb.len()].copy_from_slice(&sbb);
  fs.wsect(1, &buf);

  let rootino = fs.ialloc(T_DIR);
  assert!(rootino == ROOTINO);

  fs.iappend(rootino, &dirent(rootino, b"."));
  fs.iappend(rootino, &dirent(rootino, b".."));

  for path in &args[2..] {
    let mut shortname = Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string();

    // Skip leading _ in name when writing to file system.
    // The binaries may be named _rm, _cat, etc. to keep the
    // build operating system from trying to execute them
    // in place of system binaries like rm and cat.
    if let Some(s) = shortname.strip_prefix('_') {
      shortname = s.to_string();
    }

    if shortname.len() > DIRSIZ {
      eprintln!("mkfs: name too long: {}", shortname);
      exit(1);
    }

    let data = match fs::read(path) {
      Ok(d) => d,
      Err(e) => {
        eprintln!("{}: {}", path, e);
        exit(1);
      }
    };

    let inum = fs.ialloc(T_FILE);
    fs.iappend(rootino, &dirent(inum, shortname.as_bytes()));
    fs.iappend(inum, &data);
  }

  // fix size of root inode dir
  let mut din = fs.rinode(rootino);
  let off = ((din.size as usize / BSIZE) + 1) * BSIZE;
  din.size = off as u32;
  fs.winode(rootino, &din);

  let used = fs.freeblock;
  fs.balloc(used);

  if let Err(e) = fs::write(&args[1], &fs.img) {
    eprintln!("{}: {}", args[1], e);
    exit(1);
  }
}
