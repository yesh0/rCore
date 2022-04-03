use riscv_decode::{*, Instruction::*, CompressedInstruction::*};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use trapframe::TrapFrame;

use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt, virt_to_phys};
use super::super::SingleStepType::{self, *};

// use frame allocator so that it's easier to handle access permissions (execute)
// and there's no need to worry about alignment
fn alloc_insn_buffer() -> usize {
    phys_to_virt(alloc_frame().unwrap())
    // TODO: make this page executable
}

fn free_insn_buffer(addr: usize) {
    dealloc_frame(virt_to_phys(addr))
}

fn byte_copy(dst_addr: usize, src_addr: usize, len: usize) {
    let src = unsafe { from_raw_parts(src_addr as *const u8, len) };
    let mut dst = unsafe { from_raw_parts_mut(dst_addr as *mut u8, len) };
    dst.copy_from_slice(src);
}

pub struct InstructionBuffer {
    addr: usize,
}

impl InstructionBuffer {
    pub fn new() -> Self {
        let addr = alloc_insn_buffer();
        Self { addr }
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn copy_in(&self, offset: usize, src_addr: usize, len: usize) {
        byte_copy(self.addr + offset, src_addr, len);
    }

    pub fn copy_out(&self, offset: usize, dst_addr: usize, len: usize) {
        byte_copy(dst_addr, self.addr + offset, len);
    }

    pub fn add_breakpoint(&self, offset: usize) {
        inject_breakpoints(self.addr + offset, None);
    }
}

impl Drop for InstructionBuffer {
    fn drop(&mut self) {
        free_insn_buffer(self.addr)
    }
}

// arch related helper functions

pub const BREAKPOINT_LEN: usize = 2;
pub fn inject_breakpoints(addr: usize, length: Option<usize>) {
    let cebreak = 0x9002 as u16; // C.EBREAK
    let bp_len = BREAKPOINT_LEN;

    let bp_count = match length {
        Some(len) => {
            assert!(len % bp_len == 0);
            len / bp_len
        }
        None => 1,
    };
    for i in 0..bp_count {
        byte_copy(addr + i * bp_len, (&cebreak as *const u16) as usize, bp_len);
    }
}

pub fn invalidate_icache() {
    unsafe {
        llvm_asm!("fence.i");
    }
}

pub fn get_insn_length(addr: usize) -> usize {
    let i = unsafe { *(addr as *const u16) };
    instruction_length(i)
}

pub fn get_insn_type(addr: usize) -> SingleStepType {
    let len = get_insn_length(addr);
    if len != 2 && len != 4 {
        return Unsupported;
    }

    let i = unsafe { *(addr as *const u32) };
    match decode(i) {
        Ok(insn) => {
            match insn {
                Auipc(_) | Jal(_) | Jalr(_) | Beq(_) |
                Bne(_) | Blt(_) | Bge(_) | Bltu(_) | Bgeu(_) => Emulate,
                Compressed(c_insn) => {
                    match c_insn {
                        CJ(_) | CBeqz(_) | CBnez(_) => Emulate,
                        _ => Execute,
                    }
                }
                _ => Execute, // TODO: handle priviledged instructions
            }
        }
        Err(err) => Unsupported,
    }
}

pub fn get_trapframe_pc(tf: &TrapFrame) -> usize {
    tf.sepc
}

pub fn set_trapframe_pc(tf: &mut TrapFrame, pc: usize) {
    tf.sepc = pc;
}

fn get_reg(tf: &TrapFrame, reg: u32) -> usize {
    let regs = unsafe {
        from_raw_parts(&tf.general.zero as *const usize, 32)
    };
    regs[reg as usize]
}

pub fn emulate_execution(tf: &mut TrapFrame, insn_addr: usize, pc: usize) {
    let i = unsafe { *(insn_addr as *const u32) };
    let insn = decode(i).unwrap();
    match insn {
        Jal(j_type) => {
            let offset = j_type.imm() as isize;
            tf.sepc = pc + offset as usize;
        }
        Beq(b_type) => {
            let offset = b_type.imm() as isize;
            let rs1 = get_reg(tf, b_type.rs1());
            let rs2 = get_reg(tf, b_type.rs2());
            if rs1 == rs2 {
                tf.sepc = pc + offset as usize;
            } else {
                tf.sepc = pc + 4;
            }
        }
        Bne(b_type) => {
            let offset = b_type.imm() as isize;
            let rs1 = get_reg(tf, b_type.rs1());
            let rs2 = get_reg(tf, b_type.rs2());
            if rs1 != rs2 {
                tf.sepc = pc + offset as usize;
            } else {
                tf.sepc = pc + 4;
            }
        }
        Compressed(c_insn) => {
            match c_insn {
                CJ(cj_type) => {
                    let offset = cj_type.imm() as isize;
                    tf.sepc = pc + offset as usize;
                }
                _ => panic!("emulation of this instruction is not supported")
            }
        }
        _ => panic!("emulation of this instruction is not supported")
    }
}

global_asm!(include_str!("test.S"));
