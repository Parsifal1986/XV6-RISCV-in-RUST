TARGET = riscv64gc-unknown-none-elf
MODE = release
OUT = target/$(TARGET)/$(MODE)

KERNEL = $(OUT)/kernel
MKFS = target/$(MODE)/mkfs

# user programs to put in the file system image
UPROGS = \
	cat \
	echo \
	forktest \
	grep \
	init \
	kill \
	ln \
	ls \
	mkdir \
	rm \
	sh \
	stressfs \
	usertests \
	grind \
	wc \
	zombie \
	logstress \
	forphan \
	dorphan

CARGO = cargo
CARGOFLAGS = --$(MODE)

QEMU = qemu-system-riscv64

ifndef CPUS
CPUS := 3
endif

QEMUOPTS = -machine virt -bios none -kernel $(KERNEL) -m 128M -smp $(CPUS) -nographic
QEMUOPTS += -global virtio-mmio.force-legacy=false
QEMUOPTS += -drive file=fs.img,if=none,format=raw,id=x0
QEMUOPTS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

# try to generate a unique GDB port
GDBPORT = $(shell expr `id -u` % 5000 + 25000)
# QEMU's gdb stub command line changed in 0.11
QEMUGDB = $(shell if $(QEMU) -help | grep -q '^-gdb'; \
	then echo "-gdb tcp::$(GDBPORT)"; \
	else echo "-s -p $(GDBPORT)"; fi)

all: kernel fs.img

kernel:
	$(CARGO) build $(CARGOFLAGS) -p kernel --target $(TARGET)

user:
	$(CARGO) build $(CARGOFLAGS) -p user --target $(TARGET)

mkfs:
	$(CARGO) build $(CARGOFLAGS) -p mkfs

# usertests expects a file named README.
fs.img: user mkfs README.md
	cp README.md $(OUT)/README
	$(MKFS) fs.img $(OUT)/README $(addprefix $(OUT)/,$(UPROGS))

qemu: kernel fs.img
	$(QEMU) $(QEMUOPTS)

.gdbinit: .gdbinit.tmpl-riscv
	sed "s/:1234/:$(GDBPORT)/" < $^ > $@

qemu-gdb: kernel fs.img .gdbinit
	@echo "*** Now run 'gdb' in another window." 1>&2
	$(QEMU) $(QEMUOPTS) -S $(QEMUGDB)

clean:
	$(CARGO) clean
	rm -f fs.img .gdbinit

.PHONY: all kernel user mkfs fs.img qemu qemu-gdb clean
