use core::fmt;

use x86::io::{inb, outb};

/// A port-mapped UART 16550 serial interface.
pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    /// Initializes the serial port.
    pub fn init(&self) {
        // FIXME: Initialize the serial port
        unsafe {
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x80);
            outb(self.port + 0, 0x03);
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x03);
            outb(self.port + 2, 0xc7);
            outb(self.port + 4, 0x0b);
            outb(self.port + 4, 0x1e);
            outb(self.port + 0, 0xae);
            if inb(self.port + 0) != 0xae {
                panic!("serial is faulty")
            }
            outb(self.port + 4, 0x0f);
        }
    }

    fn is_transmit_empty(&self) -> u8 {
        unsafe {
            return inb(self.port + 5) & 0x20;
        }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        // FIXME: Send a byte on the serial port
        while self.is_transmit_empty() == 0 {}
        unsafe {
            outb(self.port, data);
        }
    }

    fn serial_received(&self) -> u8 {
        unsafe {
            return inb(self.port + 5) & 1;
        }
    }

    /// Receives a byte on the serial port no wait.
    pub fn receive(&mut self) -> Option<u8> {
        // FIXME: Receive a byte on the serial port no wait
        while self.serial_received() == 0 {}
        unsafe {
            return Some(inb(self.port));
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}
