use crate::consts::{KERNEL_OFFSET, MEMORY_END, MEMORY_OFFSET, KSEG2_START};
use crate::memory::{init_heap, MemorySet, FRAME_ALLOCATOR};
use core::mem;
use log::*;
use rcore_memory::PAGE_SIZE;
use rcore_memory::paging::PageTable;
use riscv::asm::sfence_vma_all;
use riscv::register::{satp, sstatus, stval};
use super::paging::PageTableImpl;

/// Initialize the memory management module
pub fn init(dtb: usize) {
    // allow user memory access
    unsafe {
        sstatus::set_sum();
    }
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    remap_the_kernel(dtb);
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum(); // Allow user memory access
        satp::write(SATP);
        sfence_vma_all();
    }
}

fn init_frame_allocator() {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let mut ba = FRAME_ALLOCATOR.lock();
    let range = to_range(
        (end as usize) - KERNEL_OFFSET + MEMORY_OFFSET + PAGE_SIZE,
        MEMORY_END,
    );
    ba.insert(range);

    info!("frame allocator: init end");

    /// Transform memory area `[start, end)` to integer range for `FrameAllocator`
    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        assert!(page_start < page_end, "illegal range for frame allocator");
        page_start..page_end
    }
}

/// See implementation of x86-64
pub fn init_kernel_kseg2_map() {
    let mut page_table = unsafe { PageTableImpl::kernel_table() };
    // Dirty hack here:
    // We do not really need the mapping. Indeed, we only need the second-level page table.
    // Second-level page table item can then be copied to all page tables safely.
    // This hack requires the page table not to recycle the second level page table on unmap.

    page_table.map(KSEG2_START, 0x0).update();
    page_table.unmap(KSEG2_START);
}

/// Remap the kernel memory address with 4K page recorded in p1 page table
fn remap_the_kernel(_dtb: usize) {
    let mut ms = MemorySet::new_bare();
    let mut page_table = ms.get_page_table_mut();
    page_table.map_kernel_initial();
    unsafe {
        ms.activate();
        SATP = ms.token();
    }
    mem::forget(ms);
    info!("remap kernel end");
}

// First core stores its SATP here.
// Other cores load it later.
pub static mut SATP: usize = 0;

pub unsafe fn clear_bss() {
    let start = sbss as usize;
    let end = ebss as usize;
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        (i as *mut usize).write(0);
    }
}

// Symbols provided by linker script
#[allow(dead_code)]
extern "C" {
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

pub fn get_page_fault_addr() -> usize {
    stval::read()
}

pub fn set_page_table(vmtoken: usize) {
    satp::write(vmtoken);
    unsafe { sfence_vma_all() }
}
