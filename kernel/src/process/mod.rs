pub use self::structs::*;
use crate::arch::cpu;
use crate::{
    consts::{MAX_CPU_NUM, MAX_PROCESS_NUM},
    memory::phys_to_virt,
    syscall::handle_syscall,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use log::*;
use trapframe::UserContext;
use crate::sync::SpinLock;
use lazy_static::*;

mod abi;
pub mod futex;
pub mod proc;
pub mod structs;
pub mod thread;

use crate::sync::SpinNoIrqLock as Mutex;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
pub use futex::*;
pub use proc::*;
pub use structs::*;
pub use thread::*;

pub fn init() {
    // create init process
    crate::shell::add_user_shell();

    info!("process: init end");
}

lazy_static! {
    pub static ref PROCESSORS: SpinLock<Vec<Option<Arc<Thread>>>> = 
        SpinLock::new((0..MAX_CPU_NUM).map(|_| None).collect());
}

/// Get current thread
///
/// `Thread` is a thread-local object.
/// It is safe to call this once, and pass `&mut Thread` as a function argument.
///
/// Don't use it unless necessary.
pub fn current_thread() -> Option<Arc<Thread>> {
    let cpu_id = cpu::id();
    // unsafe { PROCESSORS[cpu_id].clone() }
    PROCESSORS.lock()[cpu_id].clone()
}
