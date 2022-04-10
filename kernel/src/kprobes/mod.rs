pub mod kprobes;
pub mod kretprobes;

use alloc::sync::Arc;
use kprobes::Handler;
use trapframe::TrapFrame;

#[cfg(riscv)]
#[path = "arch/riscv/mod.rs"]
mod arch;

pub fn register_kprobe(addr: usize, pre_handler: Arc<Handler>, post_handler: Option<Arc<Handler>>) {
    let _ok = kprobes::register_kprobe(addr, pre_handler, post_handler);
}

pub fn unregister_kprobe(addr: usize) {
    let _ok = kprobes::unregister_kprobe(addr);
}

pub fn breakpoint_handler(tf: &mut TrapFrame) {
    let handled = kprobes::kprobe_trap_handler(tf);
    if !handled {
        kretprobes::kretprobe_trap_handler(tf);
    }
}
