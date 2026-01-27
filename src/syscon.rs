use crate::arch;

use crate::dtb::*;
use crate::println; 

#[derive(Clone, Copy, Debug)]
struct Action{
    value: u32,
    ptr: *mut u32,
}

impl Action {
    pub const fn default() -> Self {
        Self { value: 0, ptr: core::ptr::null_mut() }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Syscon{
    poweroff: Action,
    reboot: Action,
}

static mut SYSCON: Syscon = Syscon{
    poweroff: Action::default(),
    reboot: Action::default(),
};

pub fn init(dtb: &Dtb) -> Result<(), DtbError>{
    println!("Initializing syscon");


    println!("Initializing syscon-poweroff");
    {
        let node = dtb.find_compatable_nodes(b"syscon-poweroff")?.expect_one()?;
        let props = node.properties()?;
        let handle = props.expect(b"regmap")?.u32()?;
        let offset = props.expect(b"offset")?.u32()? as usize;
        let value = props.expect(b"value")?.u32()?;

        let node = dtb.find_compatable_nodes(b"syscon")?.expect_one()?;
        let props = node.properties()?;
        let [start, _] = props.expect(b"reg")?.u64_array::<2>()?;

        if handle != props.expect(b"phandle")?.u32()?{
            return Err(DtbError::ByteStream)
        }
        
        unsafe{
            SYSCON.poweroff.ptr = (start as usize + offset) as *mut u32;
            SYSCON.poweroff.value = value;
        }
    }

    println!("Initializing syscon-reboot");
    {
        let node = dtb.find_compatable_nodes(b"syscon-reboot")?.expect_one()?;
        let props = node.properties()?;
        let handle = props.expect(b"regmap")?.u32()?;
        let offset = props.expect(b"offset")?.u32()? as usize;
        let value = props.expect(b"value")?.u32()?;

        let node = dtb.find_compatable_nodes(b"syscon")?.expect_one()?;
        let props = node.properties()?;
        let [start, _] = props.expect(b"reg")?.u64_array::<2>()?;

        if handle != props.expect(b"phandle")?.u32()?{
            return Err(DtbError::ByteStream)
        }
        
        unsafe{
            SYSCON.reboot.ptr = (start as usize + offset) as *mut u32;
            SYSCON.reboot.value = value;
        }
    }

    println!("Initialized syscon");
    println!("{:#x?}", unsafe {SYSCON});

    Ok(())
}

pub fn poweroff() -> ! {
    unsafe{
        if SYSCON.poweroff.ptr.is_null(){
            panic!("syscon poweroff not initialized")
        }
        SYSCON.poweroff.ptr.write_volatile(SYSCON.poweroff.value);
    }
    arch::halt()
}

pub fn reboot() -> ! {
    unsafe{
        if SYSCON.reboot.ptr.is_null(){
            panic!("syscon reboot not initialized")
        }
        SYSCON.reboot.ptr.write_volatile(SYSCON.reboot.value);
    }
    arch::halt()
}