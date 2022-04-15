extern crate rcore;
extern crate alloc;
extern crate log;
extern crate trapframe;

use rcore::lkm::api::lkm_api_pong;
use rcore::lkm::manager::ModuleManager;
use rcore::kprobes::register_kprobe;
// use alloc::vec::Vec;
use alloc::sync::Arc;
use log::*;
use trapframe::TrapFrame;

pub mod hello;

fn query_symbol(symbol: &str) -> Option<usize> {
    ModuleManager::with(|mm| {
        mm.resolve_symbol(symbol)
    })
}

fn trace_fork(_tf: &mut TrapFrame) {
    error!("fork called!");
}

#[no_mangle]
pub extern "C" fn init_module() {
    lkm_api_pong();
    // let mut v: Vec<u8>=Vec::new();
    // v.push(10);
    // v.push(20);
    // hello::hello_again();

    let addr = query_symbol("_RNvMNtNtCsgmabU2Qg1sx_5rcore7syscall4procNtB4_7Syscall8sys_fork").unwrap();
    register_kprobe(addr, Arc::new(trace_fork), None);
}

