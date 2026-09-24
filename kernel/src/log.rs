// Simple logging that allows concurrent FS system calls.
//
// A log transaction contains the updates of multiple FS system
// calls. The logging system only commits when there are
// no FS system calls active. Thus there is never
// any reasoning required about whether a commit might
// write an uncommitted system call's updates to disk.
//
// A system call should call begin_op()/end_op() to mark
// its start and end. Usually begin_op() just increments
// the count of in-progress FS system calls and returns.
// But if it thinks the log is close to running out, it
// sleeps until the last outstanding end_op() commits.
//
// The log is a physical re-do log containing disk blocks.
// The on-disk log format:
//   header block, containing block #s for block A, B, C, ...
//   block A
//   block B
//   block C
//   ...
// Log appends are synchronous.

use core::ptr::copy;

use crate::bio::{bpin, bread, brelse, bunpin, bwrite};
use crate::buf::Buf;
use crate::fs::{Superblock, BSIZE};
use crate::param::{LOGBLOCKS, MAXOPBLOCKS};
use crate::printf::panic;
use crate::proc::{sleep, wakeup};
use crate::spinlock::{acquire, initlock, release, Spinlock};

// Contents of the header block, used for both the on-disk header block
// and to keep track in memory of logged block# before commit.
#[repr(C)]
struct Logheader {
  n: i32,
  block: [i32; LOGBLOCKS],
}

struct Log {
  lock: Spinlock,
  start: i32,
  outstanding: i32, // how many FS sys calls are executing.
  committing: bool, // in commit(), please wait.
  dev: u32,
  lh: Logheader,
}

static mut LOG: Log = Log {
  lock: Spinlock::new(),
  start: 0,
  outstanding: 0,
  committing: false,
  dev: 0,
  lh: Logheader { n: 0, block: [0; LOGBLOCKS] },
};

fn log_chan() -> *const u8 {
  &raw const LOG as *const u8
}

pub fn initlog(dev: u32, sb: &Superblock) {
  if size_of::<Logheader>() >= BSIZE {
    panic("initlog: too big logheader");
  }

  unsafe {
    initlock(&raw mut LOG.lock, "log");
    LOG.start = sb.logstart as i32;
    LOG.dev = dev;
  }
  recover_from_log();
}

// Copy committed blocks from log to their home location
fn install_trans(recovering: bool) {
  unsafe {
    for tail in 0..LOG.lh.n as usize {
      if recovering {
        printf!("recovering tail {} dst {}\n", tail, LOG.lh.block[tail]);
      }
      let lbuf = bread(LOG.dev, (LOG.start + tail as i32 + 1) as u32); // read log block
      let dbuf = bread(LOG.dev, LOG.lh.block[tail] as u32); // read dst
      copy((*lbuf).data.as_ptr(), (*dbuf).data.as_mut_ptr(), BSIZE); // copy block to dst
      bwrite(dbuf); // write dst to disk
      if !recovering {
        bunpin(dbuf);
      }
      brelse(lbuf);
      brelse(dbuf);
    }
  }
}

// Read the log header from disk into the in-memory log header
fn read_head() {
  unsafe {
    let buf = bread(LOG.dev, LOG.start as u32);
    let lh = (*buf).data.as_ptr() as *const Logheader;
    LOG.lh.n = (*lh).n;
    for i in 0..LOG.lh.n as usize {
      LOG.lh.block[i] = (*lh).block[i];
    }
    brelse(buf);
  }
}

// Write in-memory log header to disk.
// This is the true point at which the
// current transaction commits.
fn write_head() {
  unsafe {
    let buf = bread(LOG.dev, LOG.start as u32);
    let hb = (*buf).data.as_mut_ptr() as *mut Logheader;
    (*hb).n = LOG.lh.n;
    for i in 0..LOG.lh.n as usize {
      (*hb).block[i] = LOG.lh.block[i];
    }
    bwrite(buf);
    brelse(buf);
  }
}

fn recover_from_log() {
  read_head();
  install_trans(true); // if committed, copy from log to disk
  unsafe {
    LOG.lh.n = 0;
  }
  write_head(); // clear the log
}

// called at the start of each FS system call.
pub fn begin_op() {
  unsafe {
    acquire(&raw mut LOG.lock);
    loop {
      if LOG.committing {
        sleep(log_chan(), &raw mut LOG.lock);
      } else if LOG.lh.n as usize + (LOG.outstanding as usize + 1) * MAXOPBLOCKS > LOGBLOCKS {
        // this op might exhaust log space; wait for commit.
        sleep(log_chan(), &raw mut LOG.lock);
      } else {
        LOG.outstanding += 1;
        release(&raw mut LOG.lock);
        break;
      }
    }
  }
}

// called at the end of each FS system call.
// commits if this was the last outstanding operation.
pub fn end_op() {
  let mut do_commit = false;

  unsafe {
    acquire(&raw mut LOG.lock);
    LOG.outstanding -= 1;
    if LOG.committing {
      panic("log.committing");
    }
    if LOG.outstanding == 0 {
      do_commit = true;
      LOG.committing = true;
    } else {
      // begin_op() may be waiting for log space,
      // and decrementing LOG.outstanding has decreased
      // the amount of reserved space.
      wakeup(log_chan());
    }
    release(&raw mut LOG.lock);

    if do_commit {
      // call commit w/o holding locks, since not allowed
      // to sleep with locks.
      commit();
      acquire(&raw mut LOG.lock);
      LOG.committing = false;
      wakeup(log_chan());
      release(&raw mut LOG.lock);
    }
  }
}

// Copy modified blocks from cache to log.
fn write_log() {
  unsafe {
    for tail in 0..LOG.lh.n as usize {
      let to = bread(LOG.dev, (LOG.start + tail as i32 + 1) as u32); // log block
      let from = bread(LOG.dev, LOG.lh.block[tail] as u32); // cache block
      copy((*from).data.as_ptr(), (*to).data.as_mut_ptr(), BSIZE);
      bwrite(to); // write the log
      brelse(from);
      brelse(to);
    }
  }
}

fn commit() {
  unsafe {
    if LOG.lh.n > 0 {
      write_log();      // Write modified blocks from cache to log
      write_head();     // Write header to disk -- the real commit
      install_trans(false); // Now install writes to home locations
      LOG.lh.n = 0;
      write_head();     // Erase the transaction from the log
    }
  }
}

// Caller has modified b->data and is done with the buffer.
// Record the block number and pin in the cache by increasing refcnt.
// commit()/write_log() will do the disk write.
//
// log_write() replaces bwrite(); a typical use is:
//   bp = bread(...)
//   modify bp->data[]
//   log_write(bp)
//   brelse(bp)
pub fn log_write(b: *mut Buf) {
  unsafe {
    acquire(&raw mut LOG.lock);
    if LOG.lh.n as usize >= LOGBLOCKS {
      panic("too big a transaction");
    }
    if LOG.outstanding < 1 {
      panic("log_write outside of trans");
    }

    let n = LOG.lh.n as usize;
    let mut i = 0;
    while i < n {
      if LOG.lh.block[i] == (*b).blockno as i32 {
        // log absorption
        break;
      }
      i += 1;
    }
    LOG.lh.block[i] = (*b).blockno as i32;
    if i == n {
      // Add new block to log?
      bpin(b);
      LOG.lh.n += 1;
    }
    release(&raw mut LOG.lock);
  }
}
