fn main() {
  let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

  for f in ["kernel.ld", "entry.S", "kernelvec.S", "trampoline.S", "swtch.S"] {
    println!("cargo:rerun-if-changed=src/{}", f);
  }

  println!("cargo:rustc-link-arg=-T{}/src/kernel.ld", dir);
  println!("cargo:rustc-link-arg=-zmax-page-size=4096");
}
