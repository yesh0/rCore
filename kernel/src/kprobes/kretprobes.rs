use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysResult, SysError};
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use lazy_static::*;
use crate::num::FromPrimitive;

use super::arch::{
    alloc_breakpoint, free_breakpoint, get_trapframe_pc, get_trapframe_ra, set_trapframe_pc,
    set_trapframe_ra,
};
use super::kprobes::{register_kprobe, unregister_kprobe, Handler};
use super::{KProbeArgs, KRetProbeArgs, TrapFrame};

struct KRetProbe {
    entry_handler: Option<Arc<Handler>>,
    exit_handler: Arc<Handler>,
    instance_limit: usize,
    user_data: usize,
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
        user_data: usize,
    ) -> Self {
        let instance_limit = limit.unwrap_or(usize::max_value());
        Self {
            entry_handler,
            exit_handler,
            instance_limit,
            user_data,
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

fn kretprobe_kprobe_pre_handler(tf: &mut TrapFrame, _data: usize) -> isize {
    let pc = get_trapframe_pc(tf);
    let mut kretprobes = KRETPROBES.lock();
    let probe = kretprobes.get_mut(&pc).unwrap();

    if probe.nr_instances >= probe.instance_limit {
        error!(
            "[kretprobe] number of instances for entry {:#x} reaches limit",
            pc
        );
        probe.nr_misses += 1;
        return 0;
    }

    probe.nr_instances += 1;
    if let Some(handler) = &probe.entry_handler {
        let _ = handler(tf, probe.user_data);
    }

    let ra = get_trapframe_ra(tf);
    let instance = KRetProbeInstance::new(pc, ra);
    let bp_addr = alloc_breakpoint();
    INSTANCES.lock().insert(bp_addr, instance);
    set_trapframe_ra(tf, bp_addr);
    0
}

pub fn kretprobe_trap_handler(tf: &mut TrapFrame) -> bool {
    // lock KRETPROBES first to avoid dead lock
    let mut kretprobes = KRETPROBES.lock();

    let pc = get_trapframe_pc(tf);
    let mut instance_map = INSTANCES.lock();
    let instance = instance_map.get(&pc).unwrap();

    let probe = kretprobes.get_mut(&instance.entry_addr).unwrap();
    let _ = (probe.exit_handler)(tf, probe.user_data);
    probe.nr_instances -= 1;

    let ra = instance.ret_addr;
    set_trapframe_pc(tf, ra);
    set_trapframe_ra(tf, ra);
    free_breakpoint(pc);
    instance_map.remove(&pc).unwrap();
    true
}

pub fn register_kretprobe(entry_addr: usize, args: KRetProbeArgs) -> bool {
    if !register_kprobe(entry_addr, KProbeArgs::from(kretprobe_kprobe_pre_handler)) {
        error!("[kretprobe] failed to register kprobe.");
        return false;
    }

    let probe = KRetProbe::new(
        args.exit_handler,
        args.entry_handler,
        args.limit,
        args.user_data,
    );
    KRETPROBES.lock().insert(entry_addr, probe);
    true
}

pub fn unregister_kretprobe(entry_addr: usize) -> bool {
    let mut kretprobes = KRETPROBES.lock();
    if let Some(probe) = kretprobes.get(&entry_addr) {
        if probe.nr_instances > 0 {
            error!(
                "cannot remove kretprobe for address {:#x} as it is still active",
                entry_addr
            );
            false
        } else {
            let ok = unregister_kprobe(entry_addr);
            if ok {
                kretprobes.remove(&entry_addr).unwrap();
            }
            ok
        }
    } else {
        false
    }
}

#[inline(never)]
fn recursive_fn(i: isize) -> isize {
    if i >= 5 {
        return 100;
    }

    warn!("in recursive_fn({})", i);
    return i + recursive_fn(i + 1);
}

fn test_entry_handler(tf: &mut TrapFrame, _data: usize) -> isize {
    warn!("entering fn, a0 = {}", tf.general.a0);
    0
}

fn test_exit_handler(tf: &mut TrapFrame, _data: usize) -> isize {
    warn!("exiting fn, a0 = {}", tf.general.a0);
    0
}

#[inline(never)]
fn test_sysresult(i : i32) -> SysResult {
    if i >= 10 {
        Ok(i as usize)
    } else {
        Err(SysError::from_i32(i).unwrap())
    }
}

fn test_sysresult_handler(tf: &mut TrapFrame, _data:usize) -> isize {
    let sysresult = kretprobe_recover_sysresult(tf);
    match sysresult {
        Some(sys) => warn!("[SysResult] Got {:?}", sys),
        None => warn!("[SysResult] Failed to parse from a0")
    }
    0
}

pub fn run_kretprobes_test() {
    let args = KRetProbeArgs {
        exit_handler: Arc::new(test_exit_handler),
        entry_handler: Some(Arc::new(test_entry_handler)),
        limit: None,
        user_data: 0,
    };
    register_kretprobe(recursive_fn as usize, args);
    recursive_fn(1);

    let sysresult_args = KRetProbeArgs {
        exit_handler: Arc::new(test_sysresult_handler),
        entry_handler : None,
        limit: None,
        user_data: 0,
    };
    register_kretprobe(test_sysresult as usize, sysresult_args);
    let _ = test_sysresult(4);
}

pub fn kretprobe_recover_sysresult(tf: &TrapFrame) -> Option<SysResult> {
    // recover sysresult from trapfram
    let general_ret_addr = tf.general.a0 as usize;
    let enum_flag_addr = general_ret_addr as *const u8;
    let enum_flag = unsafe { *enum_flag_addr };
    if enum_flag == 0 {
        let usize_flag = unsafe { *((general_ret_addr + 8) as *const usize) };
        Some(Ok(usize_flag))
    } else if enum_flag == 1 {
        let syserror_flag = unsafe { *((general_ret_addr + 8) as *const SysError) };
        Some(Err(syserror_flag))
    } else {
        None
    }
}
