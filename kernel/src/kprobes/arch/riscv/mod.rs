use core::slice::{from_raw_parts, from_raw_parts_mut};
use riscv_decode::{CompressedInstruction::*, Instruction::*, *};
use trapframe::TrapFrame;

use super::kprobes::SingleStepType::{self, *};
use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt, virt_to_phys};

mod breakpoint;
pub use breakpoint::*;

// use frame allocator so that it's easier to handle access permissions (execute)
// and there's no need to worry about alignment
fn alloc_insn_buffer() -> usize {
    phys_to_virt(alloc_frame().unwrap())
    // TODO: make this page executable
}

fn free_insn_buffer(addr: usize) {
    dealloc_frame(virt_to_phys(addr))
}

pub fn byte_copy(dst_addr: usize, src_addr: usize, len: usize) {
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
                Auipc(_) | Jal(_) | Jalr(_) | Beq(_) | Bne(_) | Blt(_) | Bge(_) | Bltu(_)
                | Bgeu(_) => Emulate,
                Compressed(c_insn) => match c_insn {
                    CJ(_) | CJr(_) | CJalr(_) | CBeqz(_) | CBnez(_) => Emulate,
                    _ => Execute,
                },
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

pub fn get_trapframe_ra(tf: &TrapFrame) -> usize {
    tf.general.ra
}

pub fn set_trapframe_ra(tf: &mut TrapFrame, ra: usize) {
    tf.general.ra = ra;
}

fn get_reg(tf: &TrapFrame, reg: u32) -> usize {
    let regs = unsafe { from_raw_parts(&tf.general.zero as *const usize, 32) };
    let index = reg as usize;
    if index != 0 {
        regs[index]
    } else {
        0
    }
}

fn set_reg(tf: &mut TrapFrame, reg: u32, val: usize) {
    let regs = unsafe { from_raw_parts_mut(&mut tf.general.zero as *mut usize, 32) };
    let index = reg as usize;
    if index != 0 {
        regs[index] = val;
    }
}

// converts RVC register number to common register number
// fn rvc_reg_number(i: u32) -> u32 {
//     i + 8
// }

pub fn emulate_execution(tf: &mut TrapFrame, insn_addr: usize, pc: usize) {
    let i = unsafe { *(insn_addr as *const u32) };
    let insn = decode(i).unwrap();
    match insn {
        Jal(j_type) => {
            let offset = j_type.imm() as isize;
            tf.sepc = pc + offset as usize;
            set_reg(tf, j_type.rd(), pc + 4);
        }
        Jalr(i_type) => {
            let offset = i_type.imm() as isize;
            tf.sepc = get_reg(tf, i_type.rs1()) + offset as usize;
            set_reg(tf, i_type.rd(), pc + 4);
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
        Compressed(c_insn) => match c_insn {
            CJ(cj_type) => {
                let offset = cj_type.imm() as isize;
                tf.sepc = pc + offset as usize;
            }
            CJr(cr_type) => {
                tf.sepc = get_reg(tf, cr_type.rs1());
            }
            CJalr(cr_type) => {
                tf.sepc = get_reg(tf, cr_type.rs1());
                set_reg(tf, 1, pc + 2);
            }
            _ => panic!("emulation of this instruction is not supported"),
        },
        _ => panic!("emulation of this instruction is not supported"),
    }
}

struct InstructionIterator {
    cur: usize,
    end: usize,
}

impl InstructionIterator {
    pub fn new(begin: usize, end: usize) -> Self {
        Self {
            cur: begin, end,
        }
    }
}

impl Iterator for InstructionIterator {
    type Item = (usize, Instruction);

    fn next(&mut self) -> Option<Self::Item> {
        while self.cur < self.end {
            let addr = self.cur;
            let i = unsafe { *(addr as *const u32) };
            match decode(i) {
                Ok(insn) => {
                    self.cur += instruction_length(i as u16);
                    return Some((addr, insn));
                }
                Err(err) => {
                    // TODO: different increment for different error type
                    self.cur += instruction_length(i as u16);
                }
            }
        }
        None
    }
}

// criteria: jal / jalr instrution whose rd is ra, or compressed jalr 
fn is_function_call(insn: Instruction) -> bool {
    match insn {
        Jal(j_type) if j_type.rd() == 1 => true,
        Jalr(i_type) if i_type.rd() == 1 => true,
        Compressed(c_insn) => {
            match c_insn {
                CJalr(_) => true,
                _ => false,
            }
        }
        _ => false,
    }
}

pub fn foreach_function_call(code_range: (usize, usize), action: impl FnOnce(usize) + Copy) -> usize {
    let mut count = 0;
    let mut iter = InstructionIterator::new(code_range.0, code_range.1);
    for (addr, insn) in iter {
        if is_function_call(insn) {
            action(addr);
            count = count + 1;
        }
    }
    count
}

global_asm!(include_str!("test.S"));
