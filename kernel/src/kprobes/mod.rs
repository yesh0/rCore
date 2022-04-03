pub mod kprobes;

use alloc::sync::Arc;
use trapframe::TrapFrame;

pub use kprobes::SingleStepType;

pub fn register_kprobe(addr: usize, handler: Arc<kprobes::Handler>) {
    let _ok = kprobes::register_kprobe(addr, handler);
}

pub fn unregister_kprobe(addr: usize) {
    let _ok = kprobes::unregister_kprobe(addr);
}

pub fn breakpoint_handler(tf: &mut TrapFrame) {
    let _handled = kprobes::kprobe_trap_handler(tf);
}
