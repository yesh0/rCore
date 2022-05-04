use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use trapframe::TrapFrame;

use crate::kprobes::{register_kprobe, KProbeArgs};
use crate::lkm::manager::ModuleManager;
use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysError::*, SysResult};

use super::{BpfObject::*, *};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AttachTarget {
    pub target: *const u8,
    pub prog_fd: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TracepointType {
    KProbe,
    KRetProbe,
}

// Current design is very simple and this is only intended for kprobe/kretprobe
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tracepoint {
    pub tp_type: TracepointType,
    pub token: usize,
}

lazy_static! {
    static ref ATTACHED_PROGS: Mutex<BTreeMap<Tracepoint, Vec<Arc<BpfProgram>>>> =
        Mutex::new(BTreeMap::new());
}

fn kprobe_handler(_tf: &mut TrapFrame, probed_addr: usize) -> isize {
    let tracepoint = Tracepoint {
        tp_type: TracepointType::KProbe,
        token: probed_addr,
    };

    let map = ATTACHED_PROGS.lock();
    let programs = map.get(&tracepoint).unwrap();
    for program in programs {
        let result = program.run();
        error!("run result: {}", result);
    }
    0
}

fn resolve_symbol(symbol: &str) -> Option<usize> {
    ModuleManager::with(|mm| mm.resolve_symbol(symbol))
}

pub fn bpf_program_attach(target: &str, prog_fd: u32) -> SysResult {
    // check program fd
    let program = {
        let objs = BPF_OBJECTS.lock();
        match objs.get(&prog_fd) {
            Some(Program(shared_program)) => Ok(shared_program.clone()),
            _ => Err(ENOENT),
        }
    }?;

    let pos = target.find(':').ok_or(EINVAL)?;
    let type_str = &target[0..pos];
    let fn_name = &target[(pos + 1)..];

    // determine tracepoint type
    let tp_type: TracepointType;
    if type_str.eq_ignore_ascii_case("kprobe") {
        tp_type = TracepointType::KProbe;
    } else {
        return Err(EINVAL);
    }

    match tp_type {
        TracepointType::KProbe => {
            let addr = resolve_symbol(fn_name).ok_or(ENOENT)?;
            let tracepoint = Tracepoint {
                tp_type,
                token: addr,
            };

            let mut map = ATTACHED_PROGS.lock();
            if let Some(programs) = map.get_mut(&tracepoint) {
                programs.push(program);
            } else {
                let args = KProbeArgs {
                    pre_handler: Arc::new(kprobe_handler),
                    post_handler: None,
                    user_data: addr,
                };
                let _ = register_kprobe(addr, args).ok_or(EINVAL)?;
                map.insert(tracepoint, vec![program]);
            }
        }
        _ => todo!(),
    }
    Ok(0)
}
