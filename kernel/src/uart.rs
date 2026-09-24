//
// low-level driver routines for 16550a UART.
//

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::Ordering;

use crate::console::consoleintr;
use crate::memlayout::UART0;
use crate::printf::{PANICKED, PANICKING};
use crate::proc::{sleep, wakeup};
use crate::spinlock::{acquire, initlock, pop_off, push_off, release, Spinlock};

// the UART control registers are memory-mapped
// at address UART0. this function returns the
// address of one of the registers.
#[inline(always)]
fn Reg(reg: u64) -> *mut u8 {
  (UART0 + reg) as *mut u8
}

// the UART control registers.
// some have different meanings for
// read vs write.
// see http://byterunner.com/16550.html
const RHR: u64 = 0;                  // receive holding register (for input bytes)
const THR: u64 = 0;                  // transmit holding register (for output bytes)
const IER: u64 = 1;                  // interrupt enable register
const IER_RX_ENABLE: u8 = 1 << 0;
const IER_TX_ENABLE: u8 = 1 << 1;
const FCR: u64 = 2;                  // FIFO control register
const FCR_FIFO_ENABLE: u8 = 1 << 0;
const FCR_FIFO_CLEAR: u8 = 3 << 1;   // clear the content of the two FIFOs
const ISR: u64 = 2;                  // interrupt status register
const LCR: u64 = 3;                  // line control register
const LCR_EIGHT_BITS: u8 = 3 << 0;
const LCR_BAUD_LATCH: u8 = 1 << 7;   // special mode to set baud rate
const LSR: u64 = 5;                  // line status register
const LSR_RX_READY: u8 = 1 << 0;     // input is waiting to be read from RHR
const LSR_TX_IDLE: u8 = 1 << 5;      // THR can accept another character to send

#[inline(always)]
fn ReadReg(reg: u64) -> u8 {
  unsafe { read_volatile(Reg(reg)) }
}

#[inline(always)]
fn WriteReg(reg: u64, v: u8) {
  unsafe { write_volatile(Reg(reg), v) }
}

// for transmission.
static mut TX_LOCK: Spinlock = Spinlock::new();
static mut TX_BUSY: bool = false; // is the UART busy sending?
static mut TX_CHAN: u8 = 0;       // &TX_CHAN is the "wait channel"

pub fn uartinit() {
  // disable interrupts.
  WriteReg(IER, 0x00);

  // special mode to set baud rate.
  WriteReg(LCR, LCR_BAUD_LATCH);

  // LSB for baud rate of 38.4K.
  WriteReg(0, 0x03);

  // MSB for baud rate of 38.4K.
  WriteReg(1, 0x00);

  // leave set-baud mode,
  // and set word length to 8 bits, no parity.
  WriteReg(LCR, LCR_EIGHT_BITS);

  // reset and enable FIFOs.
  WriteReg(FCR, FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

  // enable transmit and receive interrupts.
  WriteReg(IER, IER_TX_ENABLE | IER_RX_ENABLE);

  initlock(&raw mut TX_LOCK, "uart");
}

// transmit buf[] to the uart. it blocks if the
// uart is busy, so it cannot be called from
// interrupts, only from write() system calls.
pub fn uartwrite(buf: &[u8]) {
  unsafe {
    acquire(&raw mut TX_LOCK);

    for &c in buf {
      while read_volatile(&raw const TX_BUSY) {
        // wait for a UART transmit-complete interrupt
        // to set TX_BUSY to false.
        sleep(&raw const TX_CHAN, &raw mut TX_LOCK);
      }

      WriteReg(THR, c);
      TX_BUSY = true;
    }

    release(&raw mut TX_LOCK);
  }
}

// write a byte to the uart without using
// interrupts, for use by kernel printf() and
// to echo characters. it spins waiting for the uart's
// output register to be empty.
pub fn uartputc_sync(c: u8) {
  let panicking = PANICKING.load(Ordering::Relaxed);
  if !panicking {
    push_off();
  }

  if PANICKED.load(Ordering::Relaxed) {
    loop {
      core::hint::spin_loop();
    }
  }

  // wait for Transmit Holding Empty to be set in LSR.
  while ReadReg(LSR) & LSR_TX_IDLE == 0 {}
  WriteReg(THR, c);

  if !panicking {
    pop_off();
  }
}

// read one input character from the UART.
// return None if none is waiting.
fn uartgetc() -> Option<u8> {
  if ReadReg(LSR) & LSR_RX_READY != 0 {
    // input data is ready.
    Some(ReadReg(RHR))
  } else {
    None
  }
}

// handle a uart interrupt, raised because input has
// arrived, or the uart is ready for more output, or
// both. called from devintr().
pub fn uartintr() {
  ReadReg(ISR); // acknowledge the interrupt

  unsafe {
    acquire(&raw mut TX_LOCK);
    if ReadReg(LSR) & LSR_TX_IDLE != 0 {
      // UART finished transmitting; wake up sending thread.
      TX_BUSY = false;
      wakeup(&raw const TX_CHAN);
    }
    release(&raw mut TX_LOCK);
  }

  // read and process incoming characters.
  while let Some(c) = uartgetc() {
    consoleintr(c as i32);
  }
}
