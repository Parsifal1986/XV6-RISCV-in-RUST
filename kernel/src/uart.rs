use crate::memlayout::UART0;
use crate::printf::{PANICKED, PANICKING};
use crate::proc::sleep;
use crate::spinlock::{acquire, initlock, pop_off, push_off, release, Spinlock};

pub fn Reg(reg: *mut u8) -> *mut u8 {
  unsafe { reg.add(UART0 as usize) } 
}

const RHR: usize = 0;
const THR: usize = 0;
const IER: usize = 1;
const FCR: usize = 2;
const ISR: usize = 2;
const LCR: usize = 3;
const LSR: usize = 5;
const IER_RX_ENABLE: u8 = 1 << 0;
const IER_TX_ENABLE: u8 = 1 << 1;
const FCR_FIFO_ENABLE: u8 = 1 << 0;
const FCR_FIFO_CLEAR: u8 = 3 << 1;
const LCR_EIGHT_BITS: u8 = 3 << 0;
const LCR_BAUD_LATCH: u8 = 1 << 7;
const LSR_RX_READY: u8 = 1 << 0;
const LSR_TX_IDLE: u8 = 1 << 5;

pub fn read_reg(reg: *mut u8) -> u8 {
  unsafe {
    *(Reg(reg))
  }
}

pub fn write_reg(reg: *mut u8, v: u8) {
  unsafe {
    *(Reg(reg)) = v
  }
}

static mut TX_LOCK: Spinlock = Spinlock::new();
static mut TX_BUSY: i32 = 0;
static mut TX_CHAN: i32 = 0;

pub fn uartinit() {
  write_reg(IER as *mut u8, 0x00);

  write_reg(LCR as *mut u8, LCR_BAUD_LATCH);

  write_reg(0 as *mut u8, 0x03);

  write_reg(1 as *mut u8, 0x00);

  write_reg(LCR as *mut u8, 0x00);

  write_reg(FCR as *mut u8, FCR_FIFO_CLEAR | FCR_FIFO_ENABLE);

  write_reg(IER as *mut u8, IER_TX_ENABLE | IER_RX_ENABLE);

  initlock(unsafe { &mut TX_LOCK }, Some("uart"));
}

pub fn uartwrite(buf: &mut[u8], n: i32) {
  acquire(unsafe{ &mut TX_LOCK });

  let mut i = 0;

  while i < n {
    while unsafe { TX_BUSY } != 0 {
      sleep(unsafe { TX_CHAN as *mut u8 }, unsafe { &mut TX_LOCK });
    }
    
    write_reg(THR as *mut u8, buf[i as usize]);
    i += 1;
    unsafe { TX_BUSY = 1 };
  }

  release(unsafe { &mut TX_LOCK });
}

pub fn uartputc_sync(c: u8) {
  if unsafe { PANICKING } == 0 {
    push_off();
  }

  if unsafe { PANICKED } != 0 {
    loop {
        
    };
  }

  while read_reg(LSR as *mut u8) & LSR_TX_IDLE == 0 {
    
  }
  write_reg(THR as *mut u8, c);

  if unsafe { PANICKING } == 0 {
    pop_off();
  }
}

