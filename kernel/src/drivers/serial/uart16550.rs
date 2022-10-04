//! 16550 serial adapter driver for malta board

use super::SerialDriver;
use crate::drivers::device_tree::{DEVICE_TREE_INTC, DEVICE_TREE_REGISTRY};
use crate::drivers::IRQ_MANAGER;
use crate::drivers::SERIAL_DRIVERS;
use crate::drivers::{DeviceType, Driver, DRIVERS};
use crate::sync::SpinLock as Mutex;
use crate::{
    memory::phys_to_virt,
    util::{read, write},
};
use alloc::{string::String, sync::Arc};
use core::fmt::{Arguments, Result, Write};
use device_tree::Node;

pub struct SerialPort {
    base: usize,
    multiplier: usize,
}

impl Driver for SerialPort {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        if let Some(c) = self.getchar_option() {
            crate::trap::serial(c);
            super::SERIAL_ACTIVITY.notify_all();
            true
        } else {
            false
        }
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Serial
    }

    fn get_id(&self) -> String {
        format!("com_{}", self.base)
    }
}

impl SerialPort {
    fn new(base: usize, shift: usize, frequency: usize) -> SerialPort {
        let mut res = SerialPort {
            base,
            multiplier: 1 << shift,
        };
        res.init(frequency);
        res
    }

    fn write(&self, register: usize, value: u8) {
        write(self.base + register * self.multiplier, value)
    }

    fn read(&self, register: usize) -> u8 {
        read::<u8>(self.base + register * self.multiplier)
    }

    fn compute_divisors(baud_rate: usize, frequency: usize) -> [u8; 2] {
        // frequency / 16 -> max baud rate
        let divisor = (frequency / 16 / baud_rate) as u16;
        [(divisor & 0xFF) as u8, (divisor >> 8) as u8]
    }

    pub fn init(&mut self, frequency: usize) {
        // Turn off interrupts
        self.write(COM_IER, 0);
        // Set speed; requires DLAB latch
        self.write(COM_LCR, COM_LCR_DLAB);

        let divisors = SerialPort::compute_divisors(9600, frequency);
        self.write(COM_DLL, divisors[0]);
        self.write(COM_DLM, divisors[1]);

        // 8 data bits, 1 stop bit, parity off; turn off DLAB latch
        self.write(COM_LCR, COM_LCR_WLEN8 & !COM_LCR_DLAB);

        // No modem controls
        self.write(COM_MCR, 0);
        // Disable FIFO
        self.write(COM_FCR, 0);
        // Enable rcv interrupts
        self.write(COM_IER, COM_IER_RDI);
    }

    /// blocking version of putchar()
    pub fn putchar(&self, c: u8) {
        for _ in 0..100 {
            if (self.read(COM_LSR) & COM_LSR_TXRDY) == COM_LSR_TXRDY {
                break;
            }
        }
        self.write(COM_TX, c);
    }

    /// blocking version of getchar()
    pub fn getchar(&mut self) -> u8 {
        loop {
            if (self.read(COM_LSR) & COM_LSR_DATA) != 0 {
                break;
            }
        }
        let c = self.read(COM_RX);
        match c {
            255 => b'\0', // null
            c => c,
        }
    }

    /// non-blocking version of getchar()
    pub fn getchar_option(&self) -> Option<u8> {
        match self.read(COM_LSR) & COM_LSR_DATA {
            0 => None,
            _ => Some(self.read(COM_RX)),
        }
    }
}

impl SerialDriver for SerialPort {
    fn read(&self) -> u8 {
        self.getchar_option().unwrap_or(0)
    }

    fn write(&self, data: &[u8]) {
        for byte in data {
            self.putchar(*byte);
        }
    }
    fn try_read(&self) -> Option<u8> {
        self.getchar_option()
    }
}

const COM_RX: usize = 0; // In:  Receive buffer (DLAB=0)
const COM_TX: usize = 0; // Out: Transmit buffer (DLAB=0)
const COM_DLL: usize = 0; // Out: Divisor Latch Low (DLAB=1)
const COM_DLM: usize = 1; // Out: Divisor Latch High (DLAB=1)
const COM_IER: usize = 1; // Out: Interrupt Enable Register
const COM_IER_RDI: u8 = 0x01; // Enable receiver data interrupt
const COM_IIR: usize = 2; // In:  Interrupt ID Register
const COM_FCR: usize = 2; // Out: FIFO Control Register
const COM_LCR: usize = 3; // Out: Line Control Register
const COM_LCR_DLAB: u8 = 0x80; // Divisor latch access bit
const COM_LCR_WLEN8: u8 = 0x03; // Wordlength: 8 bits
const COM_MCR: usize = 4; // Out: Modem Control Register
const COM_MCR_RTS: u8 = 0x02; // RTS complement
const COM_MCR_DTR: u8 = 0x01; // DTR complement
const COM_MCR_OUT2: u8 = 0x08; // Out2 complement
const COM_LSR: usize = 5; // In:  Line Status Register
const COM_LSR_DATA: u8 = 0x01; // Data available
const COM_LSR_TXRDY: u8 = 0x20; // Transmit buffer avail
const COM_LSR_TSRE: u8 = 0x40; // Transmitter off

pub fn init_dt(dt: &Node) {
    let addr = dt.prop_usize("reg").unwrap();
    let shift = dt.prop_u32("reg-shift").unwrap_or(0) as usize;
    let frequency = dt.prop_u32("clock-frequency").unwrap_or(0x384000) as usize;
    let base = phys_to_virt(addr);
    info!("Init uart16550 at {:#x}, phys at {:#x}, shift {:#x}, freq {}", base, addr, shift, frequency);
    let com = Arc::new(SerialPort::new(base, shift, frequency));
    let mut found = false;
    let irq_opt = dt.prop_u32("interrupts").ok().map(|irq| irq as usize);
    DRIVERS.write().push(com.clone());
    SERIAL_DRIVERS.write().push(com.clone());
    if let Ok(intc) = dt.prop_u32("interrupt-parent") {
        if let Some(irq) = irq_opt {
            if let Some(manager) = DEVICE_TREE_INTC.write().get_mut(&intc) {
                manager.register_local_irq(irq, com.clone());
                info!("registered uart16550 to intc");
                info!("Init uart16550 at {:#x}, {:?}", base, dt);
                found = true;
            }
        }
    }
    if !found {
        info!("registered uart16550 to root");
        IRQ_MANAGER.write().register_opt(irq_opt, com);
    }
}

pub fn driver_init() {
    DEVICE_TREE_REGISTRY.write().insert("ns16550a", init_dt);
}
