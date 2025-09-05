use core::str::from_utf8;

pub fn panic(msg: &[u8]) -> ! {
  match from_utf8(msg) {
    Ok(s) => eprintln!("PANIC: {}", s),
    Err(_) => eprintln!("PANIC: <invalid utf8: {:?}>", msg),
  }
  loop{}
}