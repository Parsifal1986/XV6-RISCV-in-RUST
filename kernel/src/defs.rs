pub fn panic(msg: &str) -> ! {
  eprintln!("PANIC: {}", msg);
  loop{}
}