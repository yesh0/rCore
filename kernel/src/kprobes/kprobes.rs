use lazy_static::*;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use trapframe::TrapFrame;
use core::ops::FnMut;
use crate::sync::{Mutex, SpinNoIrq};

pub type Handler = dyn FnMut(&mut TrapFrame) + Sync + Send;

struct KProbe {
    addr: usize,
    pre_handler: Arc<Handler>,
    post_handler: Option<Arc<Handler>>,
}

struct KProbeInstance {

}

lazy_static! {
    static ref KPROBES: Mutex<BTreeMap<usize, KProbe>, SpinNoIrq> = Mutex::new(BTreeMap::new());
}

impl KProbe {
    pub fn new(addr: usize, pre_handler: Arc<Handler>) -> Self {
        Self {
            addr, pre_handler, post_handler: None,
        }
    }
}

pub fn register_kprobe(addr: usize, handler: Arc<Handler>) -> bool {
    let mut map = KPROBES.lock();
    if map.contains_key(&addr) {
        false
    } else {
        map.insert(addr, KProbe::new(addr, handler));
        error!("kprobe for address {:#x} inserted. {} kprobes registered", addr, map.len());
        true
    }
}

pub fn unregister_kprobe(addr: usize) -> bool {
    let mut map = KPROBES.lock();
    match map.remove(&addr) {
        Some(_) => {
            error!("unregister kprobe for address {:#x}", addr);
            true
        },
        None => false
    }
}
