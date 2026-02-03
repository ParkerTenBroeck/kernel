OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  /* QEMU virt RAM usually starts at 0x8000_0000 */
  . = 0x80000000; 

  _kernel_start = .;

  .text : ALIGN(4K) {
    _text_start = .;
    KEEP(*(.text.entry))
    *(.text .text.*)
  }
  _text_end = .;


  .rodata : ALIGN(4K) {
    _rodata_start = .;
    *(.rodata .rodata.*)
  }
  _rodata_end = .;


  .data : ALIGN(4K) {
    _data_start = .;
    *(.data .data.*)
  }
  _data_end = .;


  .bss : ALIGN(4K) {
    _bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
  }
  _bss_end = .;

  . = ALIGN(4K);
  _stack_start = .;
  PROVIDE(_stack_top = . + 0x20000); /* 128 KiB stack */

  _kernel_end = . + 0x20000;
}

