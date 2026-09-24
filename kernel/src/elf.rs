// Format of an ELF executable file

pub const ELF_MAGIC: u32 = 0x464C457F; // "\x7FELF" in little endian

// File header
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elfhdr {
  pub magic: u32, // must equal ELF_MAGIC
  pub elf: [u8; 12],
  pub typ: u16,
  pub machine: u16,
  pub version: u32,
  pub entry: u64,
  pub phoff: u64,
  pub shoff: u64,
  pub flags: u32,
  pub ehsize: u16,
  pub phentsize: u16,
  pub phnum: u16,
  pub shentsize: u16,
  pub shnum: u16,
  pub shstrndx: u16,
}

// Program section header
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Proghdr {
  pub typ: u32,
  pub flags: u32,
  pub off: u64,
  pub vaddr: u64,
  pub paddr: u64,
  pub filesz: u64,
  pub memsz: u64,
  pub align: u64,
}

// Values for Proghdr type
pub const ELF_PROG_LOAD: u32 = 1;

// Flag bits for Proghdr flags
pub const ELF_PROG_FLAG_EXEC: u32 = 1;
pub const ELF_PROG_FLAG_WRITE: u32 = 2;
pub const ELF_PROG_FLAG_READ: u32 = 4;
