pub const T_DIR: i16 = 1;
pub const T_FILE: i16 = 2;
pub const T_DEVICE: i16 = 3;

pub struct Stat {
  dev: i32,
  ino: u32,
  inode_type: i16,
  nlink: i16,
  size: u64,
}