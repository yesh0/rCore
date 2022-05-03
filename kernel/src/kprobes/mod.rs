pub mod kprobes;
pub mod kretprobes;

use alloc::sync::Arc;
use kprobes::{Handler, HandlerFn};
use trapframe::TrapFrame;

#[cfg(riscv)]
#[path = "arch/riscv/mod.rs"]
mod arch;

pub struct KProbeArgs {
    pub pre_handler: Arc<Handler>,
    pub post_handler: Option<Arc<Handler>>,
    // Extra user-defined data. Kprobes will not touch it and pass it to handler as-is.
    pub user_data: usize,
}

pub struct KRetProbeArgs {
    pub exit_handler: Arc<Handler>,
    pub entry_handler: Option<Arc<Handler>>,
    pub limit: Option<usize>,
    pub user_data: usize,
}

impl KProbeArgs {
    pub fn from(handler: HandlerFn) -> Self {
        Self {
            pre_handler: Arc::new(handler),
            post_handler: None,
            user_data: 0,
        }
    }
}

impl KRetProbeArgs {
    pub fn from(handler: HandlerFn) -> Self {
        Self {
            exit_handler: Arc::new(handler),
            entry_handler: None,
            limit: None,
            user_data: 0,
        }
    }
}

pub fn register_kprobe(addr: usize, args: KProbeArgs) -> Option<()> {
    match kprobes::register_kprobe(addr, args) {
        true => Some(()),
        false => None,
    }
}

pub fn unregister_kprobe(addr: usize) -> Option<()> {
    match kprobes::unregister_kprobe(addr) {
        true => Some(()),
        false => None,
    }
}

pub fn register_kretprobe(addr: usize, args: KRetProbeArgs) -> Option<()> {
    match kretprobes::register_kretprobe(addr, args) {
        true => Some(()),
        false => None,
    }
}

pub fn unregister_kretprobe(addr: usize) -> Option<()> {
    match kretprobes::unregister_kretprobe(addr) {
        true => Some(()),
        false => None,
    }
}

pub fn breakpoint_handler(tf: &mut TrapFrame) {
    let handled = kprobes::kprobe_trap_handler(tf);
    if !handled {
        kretprobes::kretprobe_trap_handler(tf);
    }
}
