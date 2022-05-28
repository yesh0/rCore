use crate::sync::SpinLock as Mutex;
use core::ops::DerefMut;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use lazy_static::lazy_static;

use super::arch::{get_trapframe_pc, foreach_function_call};
use super::{register_kprobe, unregister_kprobe, KProbeArgs};
use super::TrapFrame;

struct TraceSamples {
    pub depth: usize,
    pub targets: BTreeMap<usize, usize>, // function address -> depth
    pub calls: BTreeSet<usize>, // addresses of probed call instructions
}

lazy_static! {
    static ref SAMPLES: Mutex<TraceSamples> = Mutex::new(TraceSamples::new());
}

impl TraceSamples {
    pub fn new() -> Self {
        Self {
            depth: 0,
            targets: BTreeMap::new(),
            calls: BTreeSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.depth = 0;
        self.targets.clear();
    }

    pub fn unregister_all(&self) {
        for &addr in &self.calls {
            unregister_kprobe(addr).unwrap();
        }
    }

    pub fn with<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        let mut samples = SAMPLES.lock();
        f(samples.deref_mut())
    }
}

fn find_function_range(symbols: &Vec<(String, usize)>, entry: usize) -> Option<(usize, usize)> {
    let mut l: usize = 0;
    let mut r = symbols.len();
    while l < r {
        let m = l + (r - l) / 2;
        if symbols[m].1 <= entry {
            l = m + 1;
        } else {
            r = m;
        }
    }
    if r < symbols.len() {
        Some((entry, symbols[r].1))
    } else {
        None
    }
}

fn dynamic_trace_kprobe_pre_handler(tf: &mut TrapFrame, _data: usize) -> isize {
    // let pc = get_trapframe_pc(tf);
    // warn!("call source: {:#x}", pc);
    // assert pc in TraceSamples::calls
    0
}

fn dynamic_trace_kprobe_post_handler(tf: &mut TrapFrame, _data: usize) -> isize {
    let pc = get_trapframe_pc(tf);
    // warn!("call target: {:#x}", pc);
    TraceSamples::with(|samples| {
        let targets = &mut samples.targets;
        if !targets.contains_key(&pc) {
            targets.insert(pc, samples.depth + 1);
        }
    });
    0
}

pub fn trace_root_function(fn_entry: usize) {
    let range = crate::lkm::manager::ModuleManager::with(|mm| {
        let ksymbols = mm.get_kernel_symbols();
        find_function_range(ksymbols, fn_entry)
    }).unwrap();

    let count = foreach_function_call(range, |addr| {
        TraceSamples::with(|samples| {
            let args = KProbeArgs {
                pre_handler: Arc::new(dynamic_trace_kprobe_pre_handler),
                post_handler: Some(Arc::new(dynamic_trace_kprobe_post_handler)),
                user_data: samples.depth,
            };
            // register_kprobe mail fail when "range"s of different functions overlap
            if register_kprobe(addr, args).is_some() {
                samples.calls.insert(addr);
            }
        })
    });
    warn!("{} call instructions found for {:#x}", count, fn_entry);

    TraceSamples::with(|samples| {
        samples.targets.insert(fn_entry, samples.depth);
    });
}

pub fn trace_next_step() {
    let mut target_fns = Vec::new();
    TraceSamples::with(|samples| {
        samples.depth += 1;
        for (&fn_entry, &depth) in &samples.targets {
            if depth == samples.depth {
                target_fns.push(fn_entry);
            }
        }
    });

    for fn_entry in target_fns {
        trace_root_function(fn_entry);
    }
}

// tests
#[inline(never)]
fn bar(x: isize) -> isize {
    if x > 5 {
        x - 10
    } else {
        x + 2
    }
}

#[inline(never)]
fn baz(x: isize) -> isize {
    let mut y = x;
    y = bar(y);
    if x >= 10 {
        y += bar(x - 6);
    }
    x - y + bar(y)
}

#[inline(never)]
pub fn foo(n: usize) -> isize {
    let mut x = 4;
    let mut y = 0;
    for i in 0..n {
        if i % 4 == 3 {
            x += baz(x);
        }
        y += x;
        x = bar(x);
        y -= x;
    }
    y
}

pub struct Test(i32);
impl Test {
    #[inline(never)]
    pub fn l0(&mut self) { self.l1_1(); self.l1_2(); self.0 -= 1;}
    #[inline(never)]
    pub fn l1_1(&mut self) { self.l2_1(); self.l2_2(); self.0 -= 2; }
    #[inline(never)]
    pub fn l1_2(&mut self) { self.l2_3(); self.l2_4(); self.0 -= 2; }
    #[inline(never)]
    pub fn l2_1(&mut self) { self.0 += 1; }
    #[inline(never)]
    pub fn l2_2(&mut self) { self.0 += 2; }
    #[inline(never)]
    pub fn l2_3(&mut self) { self.0 += 3; }
    #[inline(never)]
    pub fn l2_4(&mut self) { self.0 += 4; }
}

pub fn run_dynamic_trace_test() {
    // println!("[1] foo(4) = {}", foo(4));
    // trace_root_function(foo as usize);
    // println!("[2] foo(4) = {}", foo(4));

    let mut t = Test(0);
    trace_root_function(Test::l0 as usize);
    t.l0();
    trace_next_step();
    t.l0();
    TraceSamples::with(|samples| {
        for (&addr, &depth) in &samples.targets {
            warn!("({:#x}, {})", addr, depth);
        }
    });
}
