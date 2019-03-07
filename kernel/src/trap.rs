use crate::process::*;
use crate::arch::interrupt::TrapFrame;
use crate::arch::cpu;
use log::*;

pub static mut TICK: usize = 0;

pub fn uptime_msec() -> usize {
    unsafe {crate::trap::TICK / crate::consts::USEC_PER_TICK / 1000}
}

pub fn timer() {
    if cpu::id() == 0 {
        unsafe { TICK += 1; }
    }
    processor().tick();
}

pub fn error(tf: &TrapFrame) -> ! {
    error!("{:#x?}", tf);
    let pid = processor().tid();
    error!("On CPU{} Process {}", cpu::id(), pid);

    processor().manager().exit(pid, 0x100);
    processor().yield_now();
    unreachable!();
}

pub fn serial(c: char) {
    if c == '\r' {
        // in linux, we use '\n' instead
        crate::fs::STDIN.push('\n');
    } else {
        crate::fs::STDIN.push(c);
    }
}