#[cfg(target_pointer_width = "64")]
type ElfAddr = u64;
#[cfg(target_pointer_width = "32")]
type ElfAddr = u32;

#[repr(C)]
#[derive(Copy, Clone)]
struct ElfRela {
    r_offset: ElfAddr,
    r_info: ElfAddr,
    r_addend: ElfAddr,
}

const R_RISCV_RELATIVE: ElfAddr = 3;

/// # Safety
/// 
/// This will invalidate pointers in static memory which have relocations on them.
/// 
/// All regions of kernel memory, even rodata must be writable at its current location.
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn relocate_kernel(target_addr: usize) {
    let current_addr: usize;
    let link_addr: usize;
    let rela_start: *const ElfRela;
    let rela_end: *const ElfRela;

    core::arch::asm!(
        "
        .option push
        .option norelax
        
        lla {0}, _kernel_start
        lga {1}, KERNEL_LINK_ADDR

        lla {2}, __rela_dyn_start
        lla {3}, __rela_dyn_end

        .option pop
        ",
        out(reg) current_addr,
        out(reg) link_addr,
        out(reg) rela_start,
        out(reg) rela_end,
    );

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    let rela = core::slice::from_raw_parts(
        rela_start,
        (rela_end as usize - rela_start as usize) / core::mem::size_of::<ElfRela>(),
    );

    let target_to_link_offset = target_addr.wrapping_sub(link_addr);
    let link_to_curr_offset = link_addr.wrapping_sub(current_addr);

    for r in rela {
        if r.r_info != R_RISCV_RELATIVE {
            continue;
        }

        let addr = (r.r_offset as usize).wrapping_sub(link_to_curr_offset) as *mut ElfAddr;

        let mut relocated_addr: usize = r.r_addend as usize;

        // Don’t relocate VDSO-like symbols linked from 0; only slide kernel-linked addresses.
        if relocated_addr >= link_addr {
            relocated_addr = relocated_addr.wrapping_add(target_to_link_offset);
        }

        addr.write_volatile(relocated_addr as ElfAddr);
    }

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    core::arch::asm!(
        "
        .option push
        .option norelax
        lla      gp, __global_pointer$
        .option pop
        "
    );

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}
