use crate::sync::SpinLock as Mutex;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use lazy_static::*;
use trapframe::TrapFrame;

use super::arch::{
    alloc_breakpoint, free_breakpoint, get_trapframe_pc, get_trapframe_ra, set_trapframe_pc,
    set_trapframe_ra,
};
use super::kprobes::{register_kprobe, Handler};

struct KRetProbe {
    entry_handler: Option<Arc<Handler>>,
    exit_handler: Arc<Handler>,
    instance_limit: usize,
    nr_instances: usize,
    nr_misses: usize,
}

struct KRetProbeInstance {
    pub entry_addr: usize, // used to obtain associated KRetProbe
    pub ret_addr: usize,
}

lazy_static! {
    static ref KRETPROBES: Mutex<BTreeMap<usize, KRetProbe>> = Mutex::new(BTreeMap::new());
    static ref INSTANCES: Mutex<BTreeMap<usize, KRetProbeInstance>> = Mutex::new(BTreeMap::new());
}

impl KRetProbe {
    pub fn new(
        exit_handler: Arc<Handler>,
        entry_handler: Option<Arc<Handler>>,
        limit: Option<usize>,
    ) -> Self {
        let instance_limit = limit.unwrap_or(usize::max_value());
        Self {
            entry_handler,
            exit_handler,
            instance_limit,
            nr_instances: 0,
            nr_misses: 0,
        }
    }
}

impl KRetProbeInstance {
    pub fn new(entry_addr: usize, ret_addr: usize) -> Self {
        Self {
            entry_addr,
            ret_addr,
        }
    }
}

fn kretprobe_kprobe_pre_handler(tf: &mut TrapFrame) {
    let pc = get_trapframe_pc(tf);
    let mut kretprobes = KRETPROBES.lock();
    let probe = kretprobes.get_mut(&pc).unwrap();

    if probe.nr_instances >= probe.instance_limit {
        error!(
            "[kretprobe] number of instances for entry {:#x} reaches limit",
            pc
        );
        probe.nr_misses += 1;
        return;
    }

    probe.nr_instances += 1;
    if let Some(handler) = &probe.entry_handler {
        handler(tf);
    }

    let ra = get_trapframe_ra(tf);
    let instance = KRetProbeInstance::new(pc, ra);
    let bp_addr = alloc_breakpoint();
    INSTANCES.lock().insert(bp_addr, instance);
    set_trapframe_ra(tf, bp_addr);
}

pub fn kretprobe_trap_handler(tf: &mut TrapFrame) -> bool {
    // lock KRETPROBES first to avoid dead lock
    let mut kretprobes = KRETPROBES.lock();

    let pc = get_trapframe_pc(tf);
    let mut instance_map = INSTANCES.lock();
    let instance = instance_map.get(&pc).unwrap();

    let probe = kretprobes.get_mut(&instance.entry_addr).unwrap();
    (probe.exit_handler)(tf);
    probe.nr_instances -= 1;

    let ra = instance.ret_addr;
    set_trapframe_pc(tf, ra);
    set_trapframe_ra(tf, ra);
    free_breakpoint(pc);
    instance_map.remove(&pc).unwrap();
    true
}

pub fn register_kretprobe(
    entry_addr: usize,
    exit_handler: Arc<Handler>,
    entry_handler: Option<Arc<Handler>>,
    limit: Option<usize>,
) -> bool {
    if !register_kprobe(entry_addr, Arc::new(kretprobe_kprobe_pre_handler), None) {
        error!("[kretprobe] failed to register kprobe.");
        return false;
    }

    let probe = KRetProbe::new(exit_handler, entry_handler, limit);
    KRETPROBES.lock().insert(entry_addr, probe);
    true
}

#[inline(never)]
fn recursive_fn(i: isize) -> isize {
    if i >= 5 {
        return 100;
    }

    warn!("in recursive_fn({})", i);
    return i + recursive_fn(i + 1);
}

fn test_entry_handler(tf: &mut TrapFrame) {
    warn!("entering fn, a0 = {}", tf.general.a0);
}

fn test_exit_handler(tf: &mut TrapFrame) {
    warn!("exiting fn, a0 = {}", tf.general.a0);
}

pub fn run_kretprobes_test() {
    register_kretprobe(
        recursive_fn as usize,
        Arc::new(test_exit_handler),
        Some(Arc::new(test_entry_handler)),
        None,
    );
    recursive_fn(1);
}
