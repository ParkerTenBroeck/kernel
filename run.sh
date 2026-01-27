# truncate -s 64M fs.img
qemu-system-riscv64 \
  -machine virt \
  -cpu rv64,zihintpause=true \
  -m 1G -smp 1 \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/release/kernel \
  -serial mon:stdio \
  -device VGA \
  -netdev user,id=net0 \
  -device virtio-net-pci,netdev=net0 \
  -drive if=none,format=raw,file=fs.img,id=drv0 \
  -device virtio-blk-pci,drive=drv0
