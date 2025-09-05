const ELF_MAGIC: u32 = 0x464C457F;

#[repr(C)]
pub struct Elfhdr {
  magic: u32,
  elf: [u8; 12],
  typ: u16,
  machine: u16,
  version: u32,
  pub(crate) entry: u64,
  pub(crate) phoff: u64,
  shoff: u64,
  flags: u32,
  ehsize: u16,
  pub(crate) phentsize: u16,
  pub(crate) phnum: u16,
  shentsize: u16,
  shnum: u16,
  shstrndx: u16,
}

#[repr(C)]
pub struct Proghdr {
  pub(crate) typ: u32,
  pub(crate) flags: u32,
  pub(crate) off: u64,
  pub(crate) vaddr: u64,
  paddr: u64,
  pub(crate) filesz: u64,
  pub(crate) memsz: u64,
  align: u64,
}

pub const ELF_PROG_LOAD: u32 = 1;
pub const ELF_PROG_FLAG_EXEC: u32 = 1;
pub const ELF_PROG_FLAG_WRITE: u32 = 2;
pub const ELF_PROG_FLAG_READ: u32 = 4;

#[inline(always)]
pub fn check_magic(elf: &Elfhdr) -> bool {
  elf.magic == ELF_MAGIC
}