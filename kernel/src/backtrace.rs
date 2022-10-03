//! Provide backtrace upon panic
use core::mem::size_of;
use core::arch::asm;

extern "C" {
    fn stext();
    fn etext();
}

/// Returns the current frame pointer or stack base pointer
#[inline(always)]
pub fn fp() -> usize {
    let ptr: usize;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("mov {}, x29" : out(reg) ptr);
    }
    #[cfg(riscv)]
    unsafe {
        asm!("mv {}, s0", out(reg) ptr);
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("mov {}, rbp", out(reg) ptr);
    }
    #[cfg(any(target_arch = "mips"))]
    unsafe {
        // read $sp
        // llvm_asm!("ori $0, $$29, 0" : "=r"(ptr));
        todo!();
    }

    ptr
}

/// Returns the current link register.or return address
#[inline(always)]
pub fn lr() -> usize {
    let ptr: usize;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("mov {}, x30", out(reg) ptr);
    }
    #[cfg(riscv)]
    unsafe {
        asm!("mv {}, ra", out(reg) ptr);
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("movq {}, 8(rbp)", out(reg) ptr);
    }

    #[cfg(target_arch = "mips")]
    unsafe {
        // llvm_asm!("ori $0, $$31, 0" : "=r"(ptr));
        todo!();
    }

    ptr
}

use alloc::vec::Vec;
use alloc::string::String;
fn find_symbol(symbols: &Vec<(String, usize)>, pc: usize) -> Option<(&str, usize)> {
    let mut l: usize = 0;
    let mut r: usize = symbols.len();
    while l < r {
        let m = l + (r - l) / 2;
        if symbols[m].1 <= pc {
            l = m + 1;
        } else {
            r = m;
        }
    }
    if r == 0 {
        return None;
    }
    // try to find demangled names (which are shorter)
    l = r - 1;
    let mut min_len = symbols[l].0.len(); 
    let mut i = l;
    while l > 0 && symbols[l].1 == symbols[l - 1].1 {
        l = l - 1;
        if min_len > symbols[l].0.len() {
            min_len = symbols[l].0.len();
            i = l;
        }
    }
    let entry = &symbols[i];
    Some((&entry.0, pc - entry.1))
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    unsafe {
        let mut current_pc = lr();
        let mut current_fp = fp();
        let mut stack_num = 0;

        // adjust sp to the top address of backtrace() function
        #[cfg(target_arch = "mips")]
        {
            let func_base = backtrace as *const isize;
            let sp_offset = (*func_base << 16) >> 16;
            current_fp = ((current_fp as isize) - sp_offset) as usize;
        }

        println!("=== BEGIN rCore stack trace ===");

        while current_pc >= stext as usize
            && current_pc <= etext as usize
            && current_fp as usize != 0
        {
            // print current backtrace
            match size_of::<usize>() {
                4 => {
                    println!(
                        "#{:02} PC: {:#010X} FP: {:#010X}",
                        stack_num,
                        current_pc - size_of::<usize>(),
                        current_fp
                    );
                }
                _ => {
                    println!(
                        "#{:02} PC: {:#018X} FP: {:#018X}",
                        stack_num,
                        current_pc - size_of::<usize>(),
                        current_fp
                    );
                }
            }

            crate::lkm::manager::ModuleManager::with(|mm| {
                let ksymbols = mm.get_kernel_symbols();
                if let Some((name, offset)) = find_symbol(ksymbols, current_pc) {
                    print!("    {}", name);
                    if offset != 0 {
                        print!(" +{:#x}", offset);
                    }
                    println!("");
                }
            });

            stack_num = stack_num + 1;
            #[cfg(riscv)]
            {
                current_fp = *(current_fp as *const usize).offset(-2);
                current_pc = *(current_fp as *const usize).offset(-1);
            }
            #[cfg(target_arch = "aarch64")]
            {
                current_fp = *(current_fp as *const usize);
                if current_fp < crate::arch::consts::KERNEL_OFFSET {
                    break;
                }
                if current_fp != 0 {
                    current_pc = *(current_fp as *const usize).offset(1);
                }
            }
            #[cfg(target_arch = "mips")]
            {
                // the prologue of function is always like:
                // main+0: 27bd____ addiu sp, sp, -____
                // main+4: afbf____ sw    ra, ____(sp)
                let mut code_ptr = current_pc as *const isize;
                code_ptr = code_ptr.offset(-1);

                // get the stack size of last function
                while (*code_ptr as usize >> 16) != 0x27bd {
                    code_ptr = code_ptr.offset(-1);
                }
                let sp_offset = (*code_ptr << 16) >> 16;
                trace!(
                    "Found addiu sp @ {:08X}({:08x}) with sp offset {}",
                    code_ptr as usize,
                    *code_ptr,
                    sp_offset
                );

                // get the return address offset of last function
                let mut last_fun_found = false;
                while (code_ptr as usize) < current_pc {
                    if (*code_ptr as usize >> 16) == 0xafbf {
                        last_fun_found = true;
                        break;
                    }
                    code_ptr = code_ptr.offset(1);
                }
                if last_fun_found {
                    // unwind stack
                    let ra_offset = (*code_ptr << 16) >> 16;
                    trace!(
                        "Found sw ra @ {:08X}({:08x}) with ra offset {}",
                        code_ptr as usize,
                        *code_ptr,
                        ra_offset
                    );
                    current_pc = *(((current_fp as isize) + ra_offset) as *const usize);
                    current_fp = ((current_fp as isize) - sp_offset) as usize;
                    trace!("New PC {:08X} FP {:08X}", current_pc, current_fp);
                    continue;
                } else {
                    trace!("No sw ra found, probably due to optimizations.");
                    break;
                }
            }
            #[cfg(target_arch = "x86_64")]
            {
                // Kernel stack at 0x0000_57ac_0000_0000 (defined in bootloader crate)
                // size = 512 pages
                current_fp = *(current_fp as *const usize).offset(0);
                use rcore_memory::PAGE_SIZE;
                if current_fp >= 0x0000_57ac_0000_0000 + 512 * PAGE_SIZE - size_of::<usize>()
                    && current_fp <= 0xffff_ff00_0000_0000
                {
                    break;
                }
                current_pc = *(current_fp as *const usize).offset(1);
            }
        }
        println!("=== END rCore stack trace ===");
    }
}
