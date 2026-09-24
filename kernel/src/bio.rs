// Buffer cache.
//
// The buffer cache is a linked list of buf structures holding
// cached copies of disk block contents.  Caching disk blocks
// in memory reduces the number of disk reads and also provides
// a synchronization point for disk blocks used by multiple processes.
//
// Interface:
// * To get a buffer for a particular disk block, call bread.
// * After changing buffer data, call bwrite to write it to disk.
// * When done with the buffer, call brelse.
// * Do not use the buffer after calling brelse.
// * Only one process at a time can use a buffer,
//     so do not keep them longer than necessary.

use crate::buf::Buf;
use crate::param::NBUF;
use crate::printf::panic;
use crate::sleeplock::{acquiresleep, holdingsleep, initsleeplock, releasesleep};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::virtio_disk::virtio_disk_rw;

struct Bcache {
  lock: Spinlock,
  buf: [Buf; NBUF],

  // Linked list of all buffers, through prev/next.
  // Sorted by how recently the buffer was used.
  // head.next is most recent, head.prev is least.
  head: Buf,
}

static mut BCACHE: Bcache = Bcache {
  lock: Spinlock::new(),
  buf: [const { Buf::new() }; NBUF],
  head: Buf::new(),
};

pub fn binit() {
  unsafe {
    initlock(&raw mut BCACHE.lock, "bcache");

    // Create linked list of buffers
    let head = &raw mut BCACHE.head;
    (*head).prev = head;
    (*head).next = head;
    for i in 0..NBUF {
      let b = &raw mut BCACHE.buf[i];
      (*b).next = (*head).next;
      (*b).prev = head;
      initsleeplock(&raw mut (*b).lock, "buffer");
      (*(*head).next).prev = b;
      (*head).next = b;
    }
  }
}

// Look through buffer cache for block on device dev.
// If not found, allocate a buffer.
// In either case, return locked buffer.
fn bget(dev: u32, blockno: u32) -> *mut Buf {
  unsafe {
    acquire(&raw mut BCACHE.lock);

    let head = &raw mut BCACHE.head;

    // Is the block already cached?
    let mut b = (*head).next;
    while b != head {
      if (*b).dev == dev && (*b).blockno == blockno {
        (*b).refcnt += 1;
        release(&raw mut BCACHE.lock);
        acquiresleep(&raw mut (*b).lock);
        return b;
      }
      b = (*b).next;
    }

    // Not cached.
    // Recycle the least recently used (LRU) unused buffer.
    let mut b = (*head).prev;
    while b != head {
      if (*b).refcnt == 0 {
        (*b).dev = dev;
        (*b).blockno = blockno;
        (*b).valid = false;
        (*b).refcnt = 1;
        release(&raw mut BCACHE.lock);
        acquiresleep(&raw mut (*b).lock);
        return b;
      }
      b = (*b).prev;
    }
  }
  panic("bget: no buffers");
}

// Return a locked buf with the contents of the indicated block.
pub fn bread(dev: u32, blockno: u32) -> *mut Buf {
  let b = bget(dev, blockno);
  unsafe {
    if !(*b).valid {
      virtio_disk_rw(b, false);
      (*b).valid = true;
    }
  }
  b
}

// Write b's contents to disk.  Must be locked.
pub fn bwrite(b: *mut Buf) {
  unsafe {
    if !holdingsleep(&raw mut (*b).lock) {
      panic("bwrite");
    }
  }
  virtio_disk_rw(b, true);
}

// Release a locked buffer.
// Move to the head of the most-recently-used list.
pub fn brelse(b: *mut Buf) {
  unsafe {
    if !holdingsleep(&raw mut (*b).lock) {
      panic("brelse");
    }

    releasesleep(&raw mut (*b).lock);

    acquire(&raw mut BCACHE.lock);
    (*b).refcnt -= 1;
    if (*b).refcnt == 0 {
      // no one is waiting for it.
      let head = &raw mut BCACHE.head;
      (*(*b).next).prev = (*b).prev;
      (*(*b).prev).next = (*b).next;
      (*b).next = (*head).next;
      (*b).prev = head;
      (*(*head).next).prev = b;
      (*head).next = b;
    }

    release(&raw mut BCACHE.lock);
  }
}

pub fn bpin(b: *mut Buf) {
  unsafe {
    acquire(&raw mut BCACHE.lock);
    (*b).refcnt += 1;
    release(&raw mut BCACHE.lock);
  }
}

pub fn bunpin(b: *mut Buf) {
  unsafe {
    acquire(&raw mut BCACHE.lock);
    (*b).refcnt -= 1;
    release(&raw mut BCACHE.lock);
  }
}
