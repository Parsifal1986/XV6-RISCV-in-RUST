fn main() {
  let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

  println!("cargo:rerun-if-changed=user.ld");

  // only the user programs (bins) are linked.
  println!("cargo:rustc-link-arg-bins=-T{}/user.ld", dir);
  println!("cargo:rustc-link-arg-bins=-zmax-page-size=4096");
}
