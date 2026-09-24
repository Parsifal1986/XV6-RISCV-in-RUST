// Memory allocator by Kernighan and Ritchie,
// The C programming Language, 2nd ed.  Section 8.7.
//
// It is also the Rust global allocator, so user programs
// can use alloc::vec::Vec, alloc::string::String, &c.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use crate::ulib::sbrk;
use crate::SBRK_ERROR;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Header {
  ptr: *mut Header,
  size: usize, // in units of Header
}

static mut BASE: Header = Header { ptr: null_mut(), size: 0 };
static mut FREEP: *mut Header = null_mut();

pub fn free(ap: *mut u8) {
  if ap.is_null() {
    return;
  }
  unsafe {
    let bp = (ap as *mut Header).sub(1);
    let mut p = FREEP;
    while !(bp > p && bp < (*p).ptr) {
      if p >= (*p).ptr && (bp > p || bp < (*p).ptr) {
        break;
      }
      p = (*p).ptr;
    }
    if bp.add((*bp).size) == (*p).ptr {
      (*bp).size += (*(*p).ptr).size;
      (*bp).ptr = (*(*p).ptr).ptr;
    } else {
      (*bp).ptr = (*p).ptr;
    }
    if p.add((*p).size) == bp {
      (*p).size += (*bp).size;
      (*p).ptr = (*bp).ptr;
    } else {
      (*p).ptr = bp;
    }
    FREEP = p;
  }
}

fn morecore(mut nu: usize) -> *mut Header {
  if nu < 4096 {
    nu = 4096;
  }
  let bytes = nu * size_of::<Header>();
  if bytes > i32::MAX as usize {
    return null_mut();
  }
  let p = sbrk(bytes as i32);
  if p == SBRK_ERROR {
    return null_mut();
  }
  unsafe {
    let hp = p as *mut Header;
    (*hp).size = nu;
    free(hp.add(1) as *mut u8);
    FREEP
  }
}

pub fn malloc(nbytes: usize) -> *mut u8 {
  let nunits = nbytes.div_ceil(size_of::<Header>()) + 1;
  unsafe {
    let mut prevp = FREEP;
    if prevp.is_null() {
      BASE.ptr = &raw mut BASE;
      FREEP = &raw mut BASE;
      prevp = FREEP;
      BASE.size = 0;
    }
    let mut p = (*prevp).ptr;
    loop {
      if (*p).size >= nunits {
        if (*p).size == nunits {
          (*prevp).ptr = (*p).ptr;
        } else {
          (*p).size -= nunits;
          p = p.add((*p).size);
          (*p).size = nunits;
        }
        FREEP = prevp;
        return p.add(1) as *mut u8;
      }
      if p == FREEP {
        p = morecore(nunits);
        if p.is_null() {
          return null_mut();
        }
      }
      prevp = p;
      p = (*p).ptr;
    }
  }
}

struct Umalloc;

unsafe impl GlobalAlloc for Umalloc {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    if layout.align() <= size_of::<Header>() {
      return malloc(layout.size());
    }
    // over-allocate, and remember the malloc()ed pointer
    // just below the aligned block.
    let p = malloc(layout.size() + layout.align());
    if p.is_null() {
      return p;
    }
    let a = (p as usize + layout.align()) & !(layout.align() - 1);
    *((a as *mut *mut u8).sub(1)) = p;
    a as *mut u8
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    if layout.align() <= size_of::<Header>() {
      free(ptr);
    } else {
      free(*((ptr as *mut *mut u8).sub(1)));
    }
  }
}

#[global_allocator]
static ALLOCATOR: Umalloc = Umalloc;
