pub mod kprobes;

use alloc::sync::Arc;
use trapframe::TrapFrame;
use kprobes::Handler;

pub use kprobes::SingleStepType;

pub fn register_kprobe(addr: usize, pre_handler: Arc<Handler>, post_handler: Option<Arc<Handler>>) {
    let _ok = kprobes::register_kprobe(addr, pre_handler, post_handler);
}

pub fn unregister_kprobe(addr: usize) {
    let _ok = kprobes::unregister_kprobe(addr);
}

pub fn breakpoint_handler(tf: &mut TrapFrame) {
    let _handled = kprobes::kprobe_trap_handler(tf);
}
