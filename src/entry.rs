core::arch::global_asm!(
    "
.section .text.entry
.globl _start
_start:
  la sp, _stack_top
  tail rust_main
"
);