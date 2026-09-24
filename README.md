# XV6-RISCV-in-RUST

A port of MIT's [xv6-riscv](https://github.com/mit-pdos/xv6-riscv) teaching
operating system to Rust: the kernel, the user programs, and `mkfs`.

The code follows the C version closely (same file names, function names and
structure, same comments) so it can be read side by side with the xv6 book.
No C compiler is needed: the few assembly files are included with
`global_asm!`.

## Layout

- `kernel/` — the kernel (`riscv64gc-unknown-none-elf`, runs on QEMU `virt`).
  Processes and scheduling, Sv39 virtual memory with lazy `sbrk`, traps and
  system calls, a buffer cache, a crash-safe log, inodes, directories, pipes,
  `exec`, and drivers for the console (16550 UART), the PLIC and the virtio
  disk.
- `user/` — the user library (`src/lib.rs`: system calls, `printf!`/`fprintf!`,
  a K&R `malloc` that is also the Rust global allocator, and `_start`) and the
  user programs in `src/bin/`: `init`, `sh`, `cat`, `echo`, `grep`, `kill`,
  `ln`, `ls`, `mkdir`, `rm`, `wc`, `zombie`, `forktest`, `stressfs`,
  `logstress`, `forphan`, `dorphan`, `grind` and `usertests`.
- `mkfs/` — builds the file system image (a host program).

## Requirements

- Rust (stable) with the `riscv64gc-unknown-none-elf` target
  (`rust-toolchain.toml` asks rustup for it)
- `qemu-system-riscv64` (7.2 or newer)
- `make`

## Build and run

```sh
make qemu          # build the kernel, the user programs and fs.img, then boot
```

At the `$ ` prompt try `ls`, `cat README.md | wc`, or `usertests -q`.
Exit QEMU with `Ctrl-a x`. `Ctrl-p` prints the process list.

Other targets: `make kernel`, `make fs.img`, `make qemu-gdb` (then run
`gdb` with the generated `.gdbinit`), `make clean`, and `make CPUS=1 qemu`.

## Differences from the C version

- Kernel stacks are 4 pages and user stacks are 4 pages (`KSTACKPAGES` and
  `USERSTACK` in `kernel/src/param.rs`), because Rust code uses somewhat more
  stack than the C version.
- `printf!` uses Rust formatting (`{}`) instead of C format strings, in both
  the kernel and user space.
- User programs define `fn main(args: &[&str]) -> i32`; returning from `main`
  exits with that status.

---

## 简介

这是 MIT 教学操作系统 xv6-riscv 的 Rust 移植，包括内核、全部用户程序和
`mkfs`，代码结构、函数名和注释与 C 版本一一对应，便于对照 xv6 教材阅读。

运行：安装 Rust（带 `riscv64gc-unknown-none-elf` 目标）和
`qemu-system-riscv64` 后执行 `make qemu`，在 `$ ` 提示符下可以运行 `ls`、
`usertests -q` 等程序，按 `Ctrl-a x` 退出 QEMU。
