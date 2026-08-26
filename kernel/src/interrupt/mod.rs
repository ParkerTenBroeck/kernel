use crate::{dtb::{ByteStream, DtbNodes, DtbProperties}, interrupt::plic::{Plic, PlicDev}, mem::Pointer};

pub mod plic;

unsafe trait InterruptHandler{
    fn handle(&self);
}


#[allow(static_mut_refs)]
pub fn init(dtb: &crate::dtb::Dtb) {
    for plic in dtb.nodes().compatible(b"riscv,plic0") {
        let [start, _size] = plic.properties().expect_value(b"reg", |stream| {
            stream.usize_cells_arr(dtb.root().addr_size_cells())
        });
        let max_int = plic
            .properties()
            .expect_value(b"riscv,ndev", ByteStream::u32);
        unsafe {
            let mut plic = PlicDev::new(Pointer::from_phys(start as *mut Plic).virt(), max_int);

            plic.clear();
        }
    }
}