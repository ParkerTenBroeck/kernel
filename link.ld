OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  /* QEMU virt RAM usually starts at 0x8000_0000 */
  . = 0x80000000; 

  .text : ALIGN(4K) {
    KEEP(*(.text.entry))
    *(.text .text.*)
  }

  .rodata : ALIGN(4K) {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(4K) {
    *(.data .data.*)
  }

  .bss : ALIGN(4K) {
    *(.bss .bss.*)
    *(COMMON)
  }

  . = ALIGN(16);
  PROVIDE(_stack_top = . + 0x20000); /* 128 KiB stack */
}

