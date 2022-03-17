mod kprobes;

use alloc::sync::Arc;

pub fn register_kprobe(addr: usize, handler: Arc<kprobes::Handler>) {
    let _ok = kprobes::register_kprobe(addr, handler);
}

pub fn unregister_kprobe(addr: usize) {
    let _ok = kprobes::unregister_kprobe(addr);
}
