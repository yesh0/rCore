use crate::sync::SpinLock as Mutex;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::ops::Fn;
use core::slice::from_raw_parts;
use lazy_static::*;
use trapframe::TrapFrame;

use super::arch::*;

pub type Handler = dyn Fn(&mut TrapFrame) + Sync + Send;

struct KProbe {
    addr: usize, // entry address
    pre_handler: Arc<Handler>,
    post_handler: Option<Arc<Handler>>,
    insn_buf: InstructionBuffer,
    insn_len: usize,
    active_count: usize,
    emulate: bool,
}

#[derive(PartialEq)]
pub enum SingleStepType {
    Unsupported,
    Execute,
    Emulate,
}

lazy_static! {
    static ref KPROBES: Mutex<BTreeMap<usize, KProbe>> = Mutex::new(BTreeMap::new());
    static ref ADDR_MAP: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
}

impl KProbe {
    pub fn new(
        addr: usize,
        pre_handler: Arc<Handler>,
        post_handler: Option<Arc<Handler>>,
        emulate: bool,
    ) -> Self {
        Self {
            addr,
            pre_handler,
            post_handler,
            insn_buf: InstructionBuffer::new(),
            insn_len: get_insn_length(addr),
            active_count: 0,
            emulate,
        }
    }

    pub fn arm(&self) {
        // write instruction buffer
        self.insn_buf.copy_in(0, self.addr, self.insn_len);
        self.insn_buf.add_breakpoint(self.insn_len);
        // replace original instruction with breakpoints
        inject_breakpoints(self.addr, Some(self.insn_len));
        invalidate_icache();
    }

    pub fn disarm(&self) {
        // change to original instruction
        self.insn_buf.copy_out(0, self.addr, self.insn_len);
        invalidate_icache();
    }
}

// returns whether this event is handled
pub fn kprobe_trap_handler(tf: &mut TrapFrame) -> bool {
    let pc = get_trapframe_pc(tf);
    let mut map = KPROBES.lock();
    if let Some(probe) = map.get_mut(&pc) {
        // breakpoint hit for the first time
        probe.active_count += 1;
        (probe.pre_handler)(tf);

        // emulate branch instructions
        if probe.emulate {
            emulate_execution(tf, probe.insn_buf.addr(), probe.addr);
            if let Some(handler) = &probe.post_handler {
                handler(tf);
            }
            probe.active_count -= 1;
            return true;
        }

        // redirect to instruction buffer (single step type is 'execute')
        // warn!("redirect target {:#x}", probe.insn_buf.addr());
        set_trapframe_pc(tf, probe.insn_buf.addr());
        return true;
    }

    if let Some(orig_addr) = ADDR_MAP.lock().get(&pc) {
        let probe = map.get_mut(orig_addr).unwrap();
        if let Some(handler) = &probe.post_handler {
            handler(tf);
        }
        probe.active_count -= 1;
        set_trapframe_pc(tf, *orig_addr + probe.insn_len);
        return true;
    }
    false
}

pub fn register_kprobe(
    addr: usize,
    pre_handler: Arc<Handler>,
    post_handler: Option<Arc<Handler>>,
) -> bool {
    let mut map = KPROBES.lock();
    if map.contains_key(&addr) {
        error!("kprobe for address {:#x} already exist", addr);
        return false;
    }

    let insn_type = get_insn_type(addr);
    if insn_type == SingleStepType::Unsupported {
        error!("target instruction is not supported");
        return false;
    }

    let emulate = insn_type == SingleStepType::Emulate;
    let probe = KProbe::new(addr, pre_handler, post_handler, emulate);
    let next_bp_addr = probe.insn_buf.addr() + probe.insn_len;
    probe.arm();

    ADDR_MAP.lock().insert(next_bp_addr, addr);
    map.insert(addr, probe);
    warn!(
        "kprobe for address {:#x} inserted. {} kprobes registered",
        addr,
        map.len()
    );
    true
}

pub fn unregister_kprobe(addr: usize) -> bool {
    let mut map = KPROBES.lock();
    if let Some(probe) = map.get(&addr) {
        if probe.active_count > 0 {
            error!(
                "cannot remove kprobe for address {:#x} as it is still active",
                addr
            );
            false
        } else {
            probe.disarm();
            map.remove(&addr).unwrap();
            true
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn kprobes_test_ok(i: usize) {
    warn!("[Kprobes test] {} OK", i);
}

extern "C" {
    fn kprobes_test_fn_count(); // *i32
    fn kprobes_test_fns(); // *u64
    fn kprobes_test_probe_points(); // *u64
}

fn test_pre_handler(_tf: &mut TrapFrame) {
    warn!("pre handler for test invoked.");
}

pub fn run_kprobes_tests() {
    unsafe {
        let nr_tests = *(kprobes_test_fn_count as *const i32) as usize;
        let test_fns = from_raw_parts(kprobes_test_fns as *const fn(usize), nr_tests);
        let probes = from_raw_parts(kprobes_test_probe_points as *const usize, nr_tests);

        for (i, &f) in test_fns.iter().enumerate() {
            register_kprobe(probes[i], Arc::new(test_pre_handler), None);
            f(0);
        }
    }
}
