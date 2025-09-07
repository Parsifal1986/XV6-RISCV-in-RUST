fn main() {
  println!("cargo:rerun-if-changed=src/kernel.ld");

  println!("cargo:rustc-link-arg=--script=src/kernel.ld");
}