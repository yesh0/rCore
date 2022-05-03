use crate::syscall::{SysError::*, SysResult};
use alloc::vec::Vec;
use xmas_elf;
use xmas_elf::header::Machine;

#[cfg(target_arch = "riscv64")]
use ebpf2rv::compile;

use super::*;
use super::consts::*;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProgramLoadExAttr {
    pub elf_prog: u64,
    pub elf_size: u32,
}

pub struct BpfProgram {
    bpf_insns: Option<Vec<u64>>,
    jited_prog: Option<Vec<u32>>, // TODO: should be something like Vec<u8>
    map_fd_table: Option<Vec<u32>>,
}

impl BpfProgram {
    // TODO: run with context
    pub fn run(&self) -> i64 {
        if let Some(compiled_code) = &self.jited_prog {
            let result = unsafe {
                type JitedFn = unsafe fn() -> i64;
                let f = core::mem::transmute::<*const u32, JitedFn>(compiled_code.as_ptr());
                f()
            };
            return result;
        }

        todo!("eBPF interpreter missing")
    }
}

// #[cfg(target_arch = "riscv64")]
pub fn bpf_program_load_ex(prog: &mut [u8]) -> SysResult {
    let base = prog.as_ptr();
    let elf = xmas_elf::ElfFile::new(prog).map_err(|_| EINVAL)?;
    match elf.header.pt2.machine().as_machine() {
        Machine::BPF => (), // machine type must be BPF
        _ => return Err(EINVAL),
    }

    let sec_hdr = elf.find_section_by_name(".text").ok_or(ENOENT)?;
    let code = sec_hdr.raw_data(&elf);
    let bpf_insns = unsafe {
        core::slice::from_raw_parts(
            code.as_ptr() as *const u64,
            code.len() / core::mem::size_of::<u64>(),
        )
    };
    let mut jit_ctx = compile::JitContext::new(bpf_insns);
    let helpers = [0u64, 0]; // TODO: make helpers
    compile::compile(&mut jit_ctx, &helpers, 256);

    let compiled_code = jit_ctx.code; // partial move
    let program = BpfProgram {
        bpf_insns: None, // currently we do not store original BPF instructions
        jited_prog: Some(compiled_code),
        map_fd_table: None,
    };
    let fd = bpf_allocate_fd();
    bpf_object_create_program(fd, program);
    Ok(fd as usize)
}

// #[cfg(not(target_arch = "riscv64"))]
// pub fn bpf_program_load_ex(prog: &mut [u8]) -> SysResult {
//     Err(EINVAL) // not supported
// }
