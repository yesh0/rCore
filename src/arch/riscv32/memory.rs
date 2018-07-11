use core::slice;
use memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, Stack};
use super::riscv::addr::*;
use ucore_memory::PAGE_SIZE;

pub fn init() {
    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_ROOT: PageData = PageData([0; PAGE_SIZE]);

    let frame = Frame::of_addr(PhysAddr::new(&PAGE_TABLE_ROOT as *const _ as u32));
    super::paging::setup_page_table(frame);
    init_frame_allocator();
    remap_the_kernel();
    init_heap();
}

fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;
    use consts::{MEMORY_OFFSET, MEMORY_END};

    let mut ba = FRAME_ALLOCATOR.lock();

    ba.insert(to_range(end as usize, MEMORY_END));
    info!("FrameAllocator init end");

    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        page_start..page_end
    }
}

fn remap_the_kernel() {
    let kstack = Stack {
        top: bootstacktop as usize,
        bottom: bootstack as usize + PAGE_SIZE,
    };
    static mut SPACE: [u8; 0x1000] = [0; 0x1000];
    let mut ms = unsafe { MemorySet::new_from_raw_space(&mut SPACE, kstack) };
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));
    unsafe { ms.activate(); }
    info!("kernel remap end");
}

// Symbols provided by linker script
extern {
    fn stext();
    fn etext();
    fn sdata();
    fn edata();
    fn srodata();
    fn erodata();
    fn sbss();
    fn ebss();
    fn start();
    fn end();
    fn bootstack();
    fn bootstacktop();
}