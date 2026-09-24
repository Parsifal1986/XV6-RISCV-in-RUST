use core::arch::asm;

// determine which hart is this
#[inline(always)]
pub fn r_mhartid() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, mhartid",
      out(reg) x
    );
  }
  x
}

pub const MSTATUS_MPP_MASK: u64 = 0b11 << 11;
pub const MSTATUS_MPP_M: u64 = 0b11 << 11;
pub const MSTATUS_MPP_S: u64 = 0b01 << 11;
pub const MSTATUS_MPP_U: u64 = 0b00 << 11;

#[inline(always)]
pub fn r_mstatus() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, mstatus",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_mstatus(x: u64) {
  unsafe {
    asm!(
      "csrw mstatus, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn w_mepc(x: u64) {
  unsafe {
    asm!(
      "csrw mepc, {0}",
      in(reg) x
    );
  }
}

pub const SSTATUS_SPP : u64 = 1 << 8;
pub const SSTATUS_SPIE : u64 = 1 << 5;
pub const SSTATUS_UPIE : u64 = 1 << 4;
pub const SSTATUS_SIE : u64 = 1 << 1;
pub const SSTATUS_UIE : u64 = 1 << 0;

#[inline(always)]
pub fn r_sstatus() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, sstatus",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_sstatus(x: u64) {
  unsafe {
    asm!(
      "csrw sstatus, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_sip() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, sip",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_sip(x: u64) {
  unsafe {
    asm!(
      "csrw sip, {0}",
      in(reg) x
    );
  }
}

pub const SIE_SEIE : u64 = 1 << 9;
pub const SIE_STIE : u64 = 1 << 5;

#[inline(always)]
pub fn r_sie() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, sie",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_sie(x: u64) {
  unsafe {
    asm!(
      "csrw sie, {0}",
      in(reg) x
    );
  }
}

pub const MIE_STIE: u64 = 1 << 5;

#[inline(always)]
pub fn r_mie() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, mie",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_mie(x: u64) {
  unsafe {
    asm!(
      "csrw mie, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_sepc() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, sepc",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_sepc(x: u64) {
  unsafe {
    asm!(
      "csrw sepc, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_medeleg() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, medeleg",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_medeleg(x: u64) {
  unsafe {
    asm!(
      "csrw medeleg, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_mideleg() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, mideleg",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_mideleg(x: u64) {
  unsafe {
    asm!(
      "csrw mideleg, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn w_stvec(x: u64) {
  unsafe {
    asm!(
      "csrw stvec, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_stvec() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, stvec",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn r_stimecmp() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, stimecmp",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_stimecmp(x: u64) {
  unsafe {
    asm!(
      "csrw stimecmp, {0}",
      in(reg) x
    );
  }
}

// Machine Environment Configuration Register
pub const MENVCFG_STCE: u64 = 1 << 63; // enable the sstc extension (stimecmp)
pub const MENVCFG_ADUE: u64 = 1 << 61; // hardware updates of PTE A/D bits

#[inline(always)]
pub fn r_menvcfg() -> u64 {
  let x : u64;
  unsafe {
    asm!(
      "csrr {0}, menvcfg",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_menvcfg(x: u64) {
  unsafe {
    asm!(
      "csrw menvcfg, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn w_pmpcfg0(x: u64) {
  unsafe {
    asm!(
      "csrw pmpcfg0, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn w_pmpaddr0(x: u64) {
  unsafe {
    asm!(
      "csrw pmpaddr0, {0}",
      in(reg) x
    );
  }
}

pub const SATP_SV39: u64 = 8 << 60;

#[inline(always)]
pub const fn MAKE_SATP(pagetable: u64) -> u64 {
  SATP_SV39 | (pagetable >> 12)
}

#[inline(always)]
pub fn r_satp() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "csrr {0}, satp",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_satp(x: u64) {
  unsafe {
    asm!(
      "csrw satp, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_scause() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "csrr {0}, scause",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn r_stval() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "csrr {0}, stval",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn r_mcounteren() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "csrr {0}, mcounteren",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_mcounteren(x: u64) {
  unsafe {
    asm!(
      "csrw mcounteren, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_time() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "csrr {0}, time",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn intr_on() {
  w_sstatus(r_sstatus() | SSTATUS_SIE);
}

#[inline(always)]
pub fn intr_off() {
  w_sstatus(r_sstatus() & !SSTATUS_SIE);
}

// are device interrupts enabled?
#[inline(always)]
pub fn intr_get() -> bool {
  (r_sstatus() & SSTATUS_SIE) != 0
}

#[inline(always)]
pub fn r_sp() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "mv {0}, sp",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn r_tp() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "mv {0}, tp",
      out(reg) x
    );
  }
  x
}

#[inline(always)]
pub fn w_tp(x: u64) {
  unsafe {
    asm!(
      "mv tp, {0}",
      in(reg) x
    );
  }
}

#[inline(always)]
pub fn r_ra() -> u64 {
  let x: u64;
  unsafe {
    asm!(
      "mv {0}, ra",
      out(reg) x
    );
  }
  x
}

// flush the TLB.
#[inline(always)]
pub fn sfence_vma() {
  // the zero, zero means flush all TLB entries.
  unsafe {
    asm!(
      "sfence.vma zero, zero"
    );
  }
}

// wait for an interrupt.
#[inline(always)]
pub fn wfi() {
  unsafe {
    asm!(
      "wfi"
    );
  }
}

// order all memory and device I/O accesses.
#[inline(always)]
pub fn fence_iorw() {
  unsafe {
    asm!(
      "fence iorw, iorw"
    );
  }
}

pub type PagetableT = *mut u64; // 512 PTEs
pub type PteT = u64;

pub const PGSIZE: u64 = 4096;
pub const PGSHIFT: u64 = 12;

#[inline(always)]
pub const fn PGROUNDUP(sz: u64) -> u64 {
  (sz + PGSIZE - 1) & !(PGSIZE - 1)
}

#[inline(always)]
pub const fn PGROUNDDOWN(a: u64) -> u64 {
  a & !(PGSIZE - 1)
}

pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;
pub const PTE_U: u64 = 1 << 4;

#[inline(always)]
pub const fn PA2PTE(pa: u64) -> u64 {
  (pa >> 12) << 10
}

#[inline(always)]
pub const fn PTE2PA(pte: u64) -> u64 {
  (pte >> 10) << 12
}

#[inline(always)]
pub const fn PTE_FLAGS(pte: u64) -> u64 {
  pte & 0x3FF
}

pub const PXMASK: u64 = 0x1FF;

#[inline(always)]
pub const fn PXSHIFT(level: u64) -> u64 {
  level * 9 + PGSHIFT
}

#[inline(always)]
pub const fn PX(level: u64, va: u64) -> u64 {
  (va >> PXSHIFT(level)) & PXMASK
}

pub const MAXVA: u64 = 1 << (9 + 9 + 9 + 12 - 1);