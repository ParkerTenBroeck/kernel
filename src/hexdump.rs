use crate::{print, println};

const BYTES_PER_ROW: usize = 16;

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";



/// # Safety
/// caller must ensure `ptr..ptr+len` is valid to read.
pub unsafe fn hexdump_u8(ptr: *const u8, len: usize) {
    let base = ptr as usize;

    #[inline(always)]
    fn color(b: u8) -> &'static str{
        match b{
            0x00 => "",
            0x0a|0x09|0x0d => YELLOW,
            0x20..=0x7e => GREEN,
            0xff => BLUE,
            _ => RED
        }
    }
    
    #[inline(always)]
    fn printable(b: u8) -> bool {
        (0x20..=0x7e).contains(&b)
    }

    let mut off = 0usize;
    while off < len {
        let row_addr = base + off;

        // Address column
        print!("{row_addr:#016x}: ");

        for col in 0..BYTES_PER_ROW {
            if col == 8 {
                print!(" "); // mid separator
            }

            let i = off + col;
            if i < len {
                let b = unsafe {ptr.add(i).read_volatile()};
                print!("{}{BOLD}{b:02x}{RESET} ", color(b));
            } else {
                print!("   ");
            }
        }

        // ASCII column
        print!(" |");
        for col in 0..BYTES_PER_ROW {
            let i = off + col;
            if i < len {
                let b =  unsafe {ptr.add(i).read_volatile()};
                let c =  if printable(b) { b as char } else { '.' };
                print!("{}{BOLD}{c}{RESET}", color(b));
            } else {
                print!(" ");
            }
        }
        println!("|");

        off += BYTES_PER_ROW;
    }
}


pub unsafe fn hexdump_u16(ptr: *const u16, len: usize) {
    let base = ptr as usize;

    #[inline(always)]
    fn color(b: u8) -> &'static str{
        match b{
            0x00 => "",
            0x0a|0x09|0x0d => YELLOW,
            0x20..=0x7e => GREEN,
            0xff => BLUE,
            _ => RED
        }
    }
    
    #[inline(always)]
    fn printable(b: u8) -> bool {
        (0x20..=0x7e).contains(&b)
    }

    let mut off = 0usize;
    while off < len {
        let row_addr = base + off;

        // Address column
        print!("{row_addr:#016x}: ");

        for col in 0..BYTES_PER_ROW/2 {
            if col == 4 {
                print!(" "); // mid separator
            }

            let i = off + col*2;
            if i < len {
                let b = unsafe {ptr.byte_add(i).read_volatile()};
                for b in b.to_be_bytes(){
                    print!("{}{BOLD}{b:02x}{RESET}", color(b));
                }
                print!(" ");
            } else {
                print!("     ");
            }
        }

        // ASCII column
        print!(" |");
        for col in 0..BYTES_PER_ROW/2 {
            let i = off + col*2;
            if i < len {
                let b =  unsafe {ptr.byte_add(i).read_volatile()};
                for b in b.to_be_bytes(){
                    let c =  if printable(b) { b as char } else { '.' };
                    print!("{}{BOLD}{c}{RESET}", color(b));
                }
            } else {
                print!("  ");
            }
        }
        println!("|");

        off += BYTES_PER_ROW;
    }
}


pub unsafe fn hexdump_u32(ptr: *const u32, len: usize) {
    let base = ptr as usize;

    #[inline(always)]
    fn color(b: u8) -> &'static str{
        match b{
            0x00 => "",
            0x0a|0x09|0x0d => YELLOW,
            0x20..=0x7e => GREEN,
            0xff => BLUE,
            _ => RED
        }
    }
    
    #[inline(always)]
    fn printable(b: u8) -> bool {
        (0x20..=0x7e).contains(&b)
    }

    let mut off = 0usize;
    while off < len {
        let row_addr = base + off;

        // Address column
        print!("{row_addr:#016x}: ");

        for col in 0..BYTES_PER_ROW/4 {
            if col == 2 {
                print!(" "); // mid separator
            }

            let i = off + col*4;
            if i < len {
                let b = unsafe {ptr.byte_add(i).read_volatile()};
                for b in b.to_be_bytes(){
                    print!("{}{BOLD}{b:02x}{RESET}", color(b));
                }
                print!(" ");
            } else {
                print!("     ");
            }
        }

        // ASCII column
        print!(" |");
        for col in 0..BYTES_PER_ROW/4 {
            let i = off + col*4;
            if i < len {
                let b =  unsafe {ptr.byte_add(i).read_volatile()};
                for b in b.to_be_bytes(){
                    let c =  if printable(b) { b as char } else { '.' };
                    print!("{}{BOLD}{c}{RESET}", color(b));
                }
            } else {
                print!("  ");
            }
        }
        println!("|");

        off += BYTES_PER_ROW;
    }
}