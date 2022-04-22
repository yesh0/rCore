extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate rcore;
extern crate spin;
extern crate trapframe;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::time::Duration;
use lazy_static::lazy_static;
use log::*;
use rcore::arch::cpu::id as cpu_id;
use rcore::arch::timer::timer_now;
use rcore::kprobes::{kretprobes::register_kretprobe, register_kprobe};
use rcore::lkm::api::lkm_api_pong;
use rcore::lkm::manager::ModuleManager;
use rcore::syscall::check_and_clone_cstr;
use spin::Mutex;
use trapframe::TrapFrame;

pub mod hello;

static mut COUNTER: isize = 1;
lazy_static! {
    static ref TIMING: Mutex<BTreeMap<usize, Duration>> = Mutex::new(BTreeMap::new());
}

fn query_symbol(symbol: &str) -> Option<usize> {
    ModuleManager::with(|mm| mm.resolve_symbol(symbol))
}

fn trace_fork(_tf: &mut TrapFrame) {
    error!("fork called!");
}

fn trace_exec_entry(tf: &mut TrapFrame) {
    let path = check_and_clone_cstr(tf.general.a2 as *const u8).unwrap_or(String::from("<BAD>"));
    error!("exec path: {}", path);

    let mut map = TIMING.lock();
    map.insert(cpu_id(), timer_now());
}

fn trace_exec_exit(_tf: &mut TrapFrame) {
    let t2 = timer_now();
    let map = TIMING.lock();
    let t1 = map[&cpu_id()];
    error!("exec took {} us", (t2 - t1).as_micros());
}

fn trace_syscall(tf: &mut TrapFrame) {
    // riscv only!
    let a0 = tf.general.a0;
    let a1 = tf.general.a1;
    let a2 = tf.general.a2;
    let a3 = tf.general.a3;
    let a4 = tf.general.a4;
    let a5 = tf.general.a5;
    error!(
        "syscall? {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
        a0, a1, a2, a3, a4, a5
    );
}

#[no_mangle]
pub extern "C" fn init_module() {
    lkm_api_pong();
    // let mut v: Vec<u8>=Vec::new();
    // v.push(10);
    // v.push(20);
    // hello::hello_again();

    let addr1 =
        query_symbol("_RNvMNtNtCsgmabU2Qg1sx_5rcore7syscall4procNtB4_7Syscall8sys_fork").unwrap();
    register_kprobe(addr1, Arc::new(trace_fork), None);

    let addr2 =
        query_symbol("_RNvMNtNtCsgmabU2Qg1sx_5rcore7syscall4procNtB4_7Syscall8sys_exec").unwrap();
    register_kretprobe(
        addr2,
        Arc::new(trace_exec_exit),
        Some(Arc::new(trace_exec_entry)),
        None,
    );

    // let addr3: usize = 0xffffffffc0277c70;
    // register_kprobe(addr3, Arc::new(trace_syscall), None);

    error!("counter = {}", unsafe { COUNTER });
    unsafe {
        COUNTER += 1;
    }
    error!("counter = {}", unsafe { COUNTER });
}
