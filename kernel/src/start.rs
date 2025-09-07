use core::arch::asm;

use crate::main;
use crate::param::NCPU;
use crate::riscv::*;

#[no_mangle]
pub static STACK0: [u8; 4096 * NCPU as usize] = [0; 4096 * NCPU as usize];

pub fn start() {
  let mut x: u64 = r_mstatus();
  x &= !MSTATUS_MPP_MASK;
  x |= MSTATUS_MPP_S;
  w_mstatus(x);

  w_mepc(main as u64);

  w_satp(0);

  w_medeleg(0xffff);
  w_mideleg(0xffff);
  w_sie(r_sie() | SIE_SEIE | SIE_STIE);

  w_pmpaddr0(0x3fffffffffffff_u64);
  w_pmpcfg0(0xf);

  timerinit();

  let id: u64 = r_mhartid();
  w_tp(id);

  unsafe {
    asm!(
      "mret"
    )
  };
}

fn timerinit() {
  w_mie(r_mie() | MIE_STIE);

  w_menvcfg(r_menvcfg() | 1 << 63_u64);

  w_mcounteren(r_mcounteren() | 2);

  w_stimecmp(r_time() + 100000); 
}