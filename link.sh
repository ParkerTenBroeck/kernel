#!/usr/bin/env bash
set -euo pipefail

REAL_LD="riscv64-none-elf-ld"
NM="riscv64-none-elf-nm"                 
CC="riscv64-none-elf-gcc"
OUT_KSYMS_O="ksyms.o"

# Rust passes all linker args to this script.
# One of them will be "-o <output>".
args=("$@")

# ---- First link: produce the intermediate ELF (no ksyms yet) ----
"$REAL_LD" "${args[@]}"

# Find output path after "-o"
out=""
for ((i=0;i<${#args[@]};i++)); do
  if [[ "${args[$i]}" == "-o" ]]; then out="${args[$((i+1))]}"; break; fi
done
[[ -n "$out" ]] || { echo "linkwrap: couldn't find -o output"; exit 1; }

# ---- Extract symbols from the ELF ----
# Example filter: global text symbols only (you choose your policy)
# Produces lines: "address type name"
mapfile -t syms < <("$NM" -n --defined-only "$out" | awk '$2 ~ /^[Tt]$/ {print $3}')

# ---- Generate assembly with relocations into .ksymtab/.ksymtab_strings ----
tmpdir="$(mktemp -d)"
ksyms_s="$tmpdir/ksyms.S"

{
  echo '    .section .ksymtab_strings,"a"'
  i=0
  for name in "${syms[@]}"; do
    echo "__ksym_name_$i:"
    echo "    .asciz \"$name\""
    i=$((i+1))
  done

  echo '    .section .ksymtab,"a"'
  echo '    .balign 8'
  i=0
  for name in "${syms[@]}"; do
    echo "__ksym_rec_$i:"
    echo "    .quad $name"
    echo "    .quad __ksym_name_$i"
    i=$((i+1))
  done
} > "$ksyms_s"

# Assemble into an object
"$CC" -c "$ksyms_s" -o "$tmpdir/$OUT_KSYMS_O" -nostdlib

# ---- Second link: re-link, adding the generated object ----
# Add ksyms.o at the end so it gets included
"$REAL_LD" "${args[@]}" "$tmpdir/$OUT_KSYMS_O"