//
// driver for qemu's virtio disk device.
// uses qemu's mmio interface to virtio.
//
// qemu ... -drive file=fs.img,if=none,format=raw,id=x0 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
//

use core::ptr::{null_mut, read_volatile, write_bytes, write_volatile};
use core::sync::atomic::{fence, Ordering};

use crate::buf::Buf;
use crate::fs::BSIZE;
use crate::kalloc::kalloc;
use crate::memlayout::VIRTIO0;
use crate::printf::panic;
use crate::proc::{sleep, wakeup};
use crate::riscv::{fence_iorw, PGSIZE};
use crate::spinlock::{acquire, initlock, release, Spinlock};
use crate::virtio::*;

// the address of virtio mmio register r.
#[inline(always)]
fn R(r: u64) -> *mut u32 {
  (VIRTIO0 + r) as *mut u32
}

#[inline(always)]
fn read_reg(r: u64) -> u32 {
  unsafe { read_volatile(R(r)) }
}

#[inline(always)]
fn write_reg(r: u64, v: u32) {
  unsafe { write_volatile(R(r), v) }
}

#[derive(Clone, Copy)]
struct Info {
  b: *mut Buf,
  status: u8,
}

struct Disk {
  // a set (not a ring) of DMA descriptors, with which the
  // driver tells the device where to read and write individual
  // disk operations. there are NUM descriptors.
  // most commands consist of a "chain" (a linked list) of a couple of
  // these descriptors.
  desc: *mut VirtqDesc,

  // a ring in which the driver writes descriptor numbers
  // that the driver would like the device to process.  it only
  // includes the head descriptor of each chain. the ring has
  // NUM elements.
  avail: *mut VirtqAvail,

  // a ring in which the device writes descriptor numbers that
  // the device has finished processing (just the head of each chain).
  // there are NUM used ring entries.
  used: *mut VirtqUsed,

  // our own book-keeping.
  free: [bool; NUM], // is a descriptor free?
  used_idx: u16,     // we've looked this far in used[2..NUM].

  // track info about in-flight operations,
  // for use when completion interrupt arrives.
  // indexed by first descriptor index of chain.
  info: [Info; NUM],

  // disk command headers.
  // one-for-one with descriptors, for convenience.
  ops: [VirtioBlkReq; NUM],

  vdisk_lock: Spinlock,
}

static mut DISK: Disk = Disk {
  desc: null_mut(),
  avail: null_mut(),
  used: null_mut(),
  free: [false; NUM],
  used_idx: 0,
  info: [Info { b: null_mut(), status: 0 }; NUM],
  ops: [const { VirtioBlkReq { typ: 0, reserved: 0, sector: 0 } }; NUM],
  vdisk_lock: Spinlock::new(),
};

pub fn virtio_disk_init() {
  let mut status: u32 = 0;

  unsafe {
    initlock(&raw mut DISK.vdisk_lock, "virtio_disk");
  }

  if read_reg(VIRTIO_MMIO_MAGIC_VALUE) != 0x74726976
    || read_reg(VIRTIO_MMIO_VERSION) != 2
    || read_reg(VIRTIO_MMIO_DEVICE_ID) != 2
    || read_reg(VIRTIO_MMIO_VENDOR_ID) != 0x554d4551
  {
    panic("could not find virtio disk");
  }

  // reset device
  write_reg(VIRTIO_MMIO_STATUS, status);

  // set ACKNOWLEDGE status bit
  status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
  write_reg(VIRTIO_MMIO_STATUS, status);

  // set DRIVER status bit
  status |= VIRTIO_CONFIG_S_DRIVER;
  write_reg(VIRTIO_MMIO_STATUS, status);

  // negotiate features
  let mut features = read_reg(VIRTIO_MMIO_DEVICE_FEATURES);
  features &= !(1 << VIRTIO_BLK_F_RO);
  features &= !(1 << VIRTIO_BLK_F_SCSI);
  features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
  features &= !(1 << VIRTIO_BLK_F_MQ);
  features &= !(1 << VIRTIO_F_ANY_LAYOUT);
  features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
  features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
  write_reg(VIRTIO_MMIO_DRIVER_FEATURES, features);

  // tell device that feature negotiation is complete.
  status |= VIRTIO_CONFIG_S_FEATURES_OK;
  write_reg(VIRTIO_MMIO_STATUS, status);

  // re-read status to ensure FEATURES_OK is set.
  status = read_reg(VIRTIO_MMIO_STATUS);
  if status & VIRTIO_CONFIG_S_FEATURES_OK == 0 {
    panic("virtio disk FEATURES_OK unset");
  }

  // initialize queue 0.
  write_reg(VIRTIO_MMIO_QUEUE_SEL, 0);

  // ensure queue 0 is not in use.
  if read_reg(VIRTIO_MMIO_QUEUE_READY) != 0 {
    panic("virtio disk should not be ready");
  }

  // check maximum queue size.
  let max = read_reg(VIRTIO_MMIO_QUEUE_NUM_MAX);
  if max == 0 {
    panic("virtio disk has no queue 0");
  }
  if (max as usize) < NUM {
    panic("virtio disk max queue too short");
  }

  unsafe {
    // allocate and zero queue memory.
    DISK.desc = kalloc() as *mut VirtqDesc;
    DISK.avail = kalloc() as *mut VirtqAvail;
    DISK.used = kalloc() as *mut VirtqUsed;
    if DISK.desc.is_null() || DISK.avail.is_null() || DISK.used.is_null() {
      panic("virtio disk kalloc");
    }
    write_bytes(DISK.desc as *mut u8, 0, PGSIZE as usize);
    write_bytes(DISK.avail as *mut u8, 0, PGSIZE as usize);
    write_bytes(DISK.used as *mut u8, 0, PGSIZE as usize);

    // set queue size.
    write_reg(VIRTIO_MMIO_QUEUE_NUM, NUM as u32);

    // write physical addresses.
    write_reg(VIRTIO_MMIO_QUEUE_DESC_LOW, DISK.desc as u64 as u32);
    write_reg(VIRTIO_MMIO_QUEUE_DESC_HIGH, (DISK.desc as u64 >> 32) as u32);
    write_reg(VIRTIO_MMIO_DRIVER_DESC_LOW, DISK.avail as u64 as u32);
    write_reg(VIRTIO_MMIO_DRIVER_DESC_HIGH, (DISK.avail as u64 >> 32) as u32);
    write_reg(VIRTIO_MMIO_DEVICE_DESC_LOW, DISK.used as u64 as u32);
    write_reg(VIRTIO_MMIO_DEVICE_DESC_HIGH, (DISK.used as u64 >> 32) as u32);

    // queue is ready.
    write_reg(VIRTIO_MMIO_QUEUE_READY, 0x1);

    // all NUM descriptors start out unused.
    for i in 0..NUM {
      DISK.free[i] = true;
    }
  }

  // tell device we're completely ready.
  status |= VIRTIO_CONFIG_S_DRIVER_OK;
  write_reg(VIRTIO_MMIO_STATUS, status);

  // plic.rs and trap.rs arrange for interrupts from VIRTIO0_IRQ.
}

// find a free descriptor, mark it non-free, return its index.
fn alloc_desc() -> Option<usize> {
  unsafe {
    for i in 0..NUM {
      if DISK.free[i] {
        DISK.free[i] = false;
        return Some(i);
      }
    }
  }
  None
}

// mark a descriptor as free.
fn free_desc(i: usize) {
  unsafe {
    if i >= NUM {
      panic("free_desc 1");
    }
    if DISK.free[i] {
      panic("free_desc 2");
    }
    let d = DISK.desc.add(i);
    (*d).addr = 0;
    (*d).len = 0;
    (*d).flags = 0;
    (*d).next = 0;
    DISK.free[i] = true;
    wakeup(&raw const DISK.free[0] as *const u8);
  }
}

// free a chain of descriptors.
fn free_chain(mut i: usize) {
  loop {
    let (flag, nxt) = unsafe {
      let d = DISK.desc.add(i);
      ((*d).flags, (*d).next)
    };
    free_desc(i);
    if flag & VRING_DESC_F_NEXT != 0 {
      i = nxt as usize;
    } else {
      break;
    }
  }
}

// allocate three descriptors (they need not be contiguous).
// disk transfers always use three descriptors.
fn alloc3_desc(idx: &mut [usize; 3]) -> i32 {
  for i in 0..3 {
    match alloc_desc() {
      Some(d) => idx[i] = d,
      None => {
        for j in 0..i {
          free_desc(idx[j]);
        }
        return -1;
      }
    }
  }
  0
}

pub fn virtio_disk_rw(b: *mut Buf, write: bool) {
  unsafe {
    let sector = (*b).blockno as u64 * (BSIZE / 512) as u64;

    acquire(&raw mut DISK.vdisk_lock);

    // the spec's Section 5.2 says that legacy block operations use
    // three descriptors: one for type/reserved/sector, one for the
    // data, one for a 1-byte status result.

    // allocate the three descriptors.
    let mut idx = [0usize; 3];
    loop {
      if alloc3_desc(&mut idx) == 0 {
        break;
      }
      sleep(&raw const DISK.free[0] as *const u8, &raw mut DISK.vdisk_lock);
    }

    // format the three descriptors.
    // qemu's virtio-blk.c reads them.

    let buf0 = &raw mut DISK.ops[idx[0]];

    (*buf0).typ = if write {
      VIRTIO_BLK_T_OUT // write the disk
    } else {
      VIRTIO_BLK_T_IN // read the disk
    };
    (*buf0).reserved = 0;
    (*buf0).sector = sector;

    let d0 = DISK.desc.add(idx[0]);
    (*d0).addr = buf0 as u64;
    (*d0).len = size_of::<VirtioBlkReq>() as u32;
    (*d0).flags = VRING_DESC_F_NEXT;
    (*d0).next = idx[1] as u16;

    let d1 = DISK.desc.add(idx[1]);
    (*d1).addr = (*b).data.as_ptr() as u64;
    (*d1).len = BSIZE as u32;
    (*d1).flags = if write {
      0 // device reads b->data
    } else {
      VRING_DESC_F_WRITE // device writes b->data
    };
    (*d1).flags |= VRING_DESC_F_NEXT;
    (*d1).next = idx[2] as u16;

    DISK.info[idx[0]].status = 0xff; // device writes 0 on success
    let d2 = DISK.desc.add(idx[2]);
    (*d2).addr = &raw mut DISK.info[idx[0]].status as u64;
    (*d2).len = 1;
    (*d2).flags = VRING_DESC_F_WRITE; // device writes the status
    (*d2).next = 0;

    // record struct buf for virtio_disk_intr().
    write_volatile(&raw mut (*b).disk, true);
    DISK.info[idx[0]].b = b;

    // tell the device the first index in our chain of descriptors.
    let avail = DISK.avail;
    let aidx = read_volatile(&raw const (*avail).idx);
    write_volatile(&raw mut (*avail).ring[aidx as usize % NUM], idx[0] as u16);

    fence(Ordering::SeqCst);

    // tell the device another avail ring entry is available.
    write_volatile(&raw mut (*avail).idx, aidx.wrapping_add(1)); // not % NUM ...

    fence_iorw();

    write_reg(VIRTIO_MMIO_QUEUE_NOTIFY, 0); // value is queue number

    // Wait for virtio_disk_intr() to say request has finished.
    while read_volatile(&raw const (*b).disk) {
      sleep(b as *const u8, &raw mut DISK.vdisk_lock);
    }

    DISK.info[idx[0]].b = null_mut();
    free_chain(idx[0]);

    release(&raw mut DISK.vdisk_lock);
  }
}

pub fn virtio_disk_intr() {
  unsafe {
    acquire(&raw mut DISK.vdisk_lock);

    // the device won't raise another interrupt until we tell it
    // we've seen this interrupt, which the following line does.
    // this may race with the device writing new entries to
    // the "used" ring, in which case we may process the new
    // completion entries in this interrupt, and have nothing to do
    // in the next interrupt, which is harmless.
    write_reg(VIRTIO_MMIO_INTERRUPT_ACK, read_reg(VIRTIO_MMIO_INTERRUPT_STATUS) & 0x3);

    fence(Ordering::SeqCst);

    // the device increments DISK.used->idx when it
    // adds an entry to the used ring.

    let used = DISK.used;
    while DISK.used_idx != read_volatile(&raw const (*used).idx) {
      fence(Ordering::SeqCst);
      let id = read_volatile(&raw const (*used).ring[DISK.used_idx as usize % NUM].id) as usize;

      if read_volatile(&raw const DISK.info[id].status) != 0 {
        panic("virtio_disk_intr status");
      }

      let b = DISK.info[id].b;
      write_volatile(&raw mut (*b).disk, false); // disk is done with buf
      wakeup(b as *const u8);

      DISK.used_idx = DISK.used_idx.wrapping_add(1);
    }

    release(&raw mut DISK.vdisk_lock);
  }
}
