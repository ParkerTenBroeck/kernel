cargo build

qemu-system-riscv64 \
  -s -S \
  -machine virt \
  -chardev stdio,id=uart0 \
  -serial chardev:uart0  \
  -device pci-testdev \
  -monitor vc  \
  -display gtk  \
  -device VGA,id=vgadev \
  -chardev vc,id=pci_uart \
  -device pci-serial,chardev=pci_uart \
  -netdev user,id=net0 \
  -device virtio-net-pci,netdev=net0 \
  -drive if=none,format=raw,readonly=off,file=fs.img,id=drv0 \
  -device virtio-blk-pci,drive=drv0 \
  -kernel target/riscv64gc-unknown-none-elf/debug/kernel
  
