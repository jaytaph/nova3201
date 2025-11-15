use crate::devices::font::FontRam;
use crate::devices::ram::Ram;
use crate::devices::timer::Timer;
use crate::devices::uart::Uart;
use crate::devices::vram::Vram;

/// Errors that can occur during bus operations
#[derive(Debug)]
pub enum BusError {
    Misaligned(u32),
    OutOfBounds(u32),
    DeviceFault(u32),
}

/// Generic bus trait for memory-mapped I/O
pub trait Bus {
    type Error;

    fn load8(&mut self, addr: u32) -> Result<u8, Self::Error>;
    fn load32(&mut self, addr: u32) -> Result<u32, Self::Error>;
    fn store8(&mut self, addr: u32, value: u8) -> Result<(), Self::Error>;
    fn store32(&mut self, addr: u32, value: u32) -> Result<(), Self::Error>;
}

// Concrete implementation of the NovaBus
pub struct NovaBus {
    pub ram: Ram,          // General RAM
    pub vram: Vram,        // Video RAM
    pub font_ram: FontRam, // Character Font RAM
    pub timer1: Timer,     // Timer1
    pub timer2: Timer,     // Timer2 , just because
    pub uart: Uart,        // Uart
}

const RAM_BASE: u32 = 0x0000_0000;
const RAM_SIZE: u32 = 1024 * 1024;
const RAM_END: u32 = RAM_BASE + RAM_SIZE - 1;

const VRAM_BASE: u32 = 0x8000_0000;
const VRAM_SIZE: u32 = 0x0000_1000;
#[allow(unused)]
const VRAM_END: u32 = VRAM_BASE + VRAM_SIZE - 1;

const FONT_BASE: u32 = 0x8000_1000;
const FONT_SIZE: u32 = 0x0000_1000;
#[allow(unused)]
const FONT_END: u32 = FONT_BASE + FONT_SIZE - 1;

const MMIO_BASE: u32 = 0x8000_2100;
const MMIO_END: u32 = 0x8000_22FF;

// Concrete MMIO registers
const TIMER1_CTRL: u32 = 0x8000_2100; // R/W
const TIMER1_PERIOD: u32 = 0x8000_2104; // R/W
const TIMER1_COUNT: u32 = 0x8000_2108; // R
const TIMER1_RESET: u32 = 0x8000_210C; // W
const TIMER1_ACK: u32 = 0x8000_2110; // W

const TIMER2_CTRL: u32 = 0x8000_2120; // R/W
const TIMER2_PERIOD: u32 = 0x8000_2124; // R/W
const TIMER2_COUNT: u32 = 0x8000_2128; // R
const TIMER2_RESET: u32 = 0x8000_212C; // W
const TIMER2_ACK: u32 = 0x8000_2130; // W

const UART_TX: u32 = 0x8000_2200; // W    - Only low 8 bits used
const UART_STATUS: u32 = 0x8000_2204; // R/W

impl NovaBus {
    pub fn new() -> Self {
        Self {
            ram: Ram::new(RAM_SIZE as usize),
            vram: Vram::new(VRAM_SIZE as usize),
            font_ram: FontRam::new(FONT_SIZE as usize),
            timer1: Timer::new(),
            timer2: Timer::new(),
            uart: Uart::new(),
        }
    }

    fn in_range(addr: u32, base: u32, size: u32) -> bool {
        addr >= base && addr < base + size
    }

    // --- MMIO helpers --------------------------------------------------------

    fn mmio_read32(&mut self, addr: u32) -> Result<u32, BusError> {
        match addr {
            TIMER1_CTRL => Ok(self.timer1.ctrl()),
            TIMER1_PERIOD => Ok(self.timer1.period()),
            TIMER1_COUNT => Ok(self.timer1.count()),
            TIMER1_ACK => Ok(0),
            TIMER1_RESET => Ok(0),

            TIMER2_CTRL => Ok(self.timer2.ctrl()),
            TIMER2_PERIOD => Ok(self.timer2.period()),
            TIMER2_COUNT => Ok(self.timer2.count()),
            TIMER2_ACK => Ok(0),
            TIMER2_RESET => Ok(0),

            UART_STATUS => Ok(self.uart.status()),
            UART_TX => Ok(0),
            _ => Err(BusError::OutOfBounds(addr)),
        }
    }

    fn mmio_write32(&mut self, addr: u32, value: u32) -> Result<(), BusError> {
        match addr {
            TIMER1_CTRL => {
                self.timer1.set_ctrl(value);
                Ok(())
            }
            TIMER1_PERIOD => {
                self.timer1.set_period(value);
                Ok(())
            }
            TIMER1_COUNT => Ok(()),
            TIMER1_ACK => {
                self.timer1.ack_irq();
                Ok(())
            }
            TIMER1_RESET => {
                self.timer1.reset();
                Ok(())
            }

            TIMER2_CTRL => {
                self.timer2.set_ctrl(value);
                Ok(())
            }
            TIMER2_PERIOD => {
                self.timer2.set_period(value);
                Ok(())
            }
            TIMER2_COUNT => Ok(()),
            TIMER2_ACK => {
                self.timer2.ack_irq();
                Ok(())
            }
            TIMER2_RESET => {
                self.timer2.reset();
                Ok(())
            }

            UART_STATUS => {
                // usually STATUS is read-only; you might ignore writes or use for clears
                Ok(())
            }
            UART_TX => {
                // normally you'd only use store8 here, but define behavior anyway:
                let byte = (value & 0xFF) as u8;
                self.uart.write8(0, byte);
                Ok(())
            }
            _ => Err(BusError::OutOfBounds(addr)),
        }
    }

    fn mmio_read8(&mut self, addr: u32) -> Result<u8, BusError> {
        // simplest: read32 then take low byte
        let word = self.mmio_read32(addr & !3)?;
        let shift = (addr & 3) * 8;
        Ok(((word >> shift) & 0xFF) as u8)
    }

    fn mmio_write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        match addr {
            UART_TX => {
                self.uart.write8(0, value);
                Ok(())
            }
            // for others, you could do read-modify-write on the 32-bit reg if you want:
            _ => {
                let aligned = addr & !3;
                let shift = (addr & 3) * 8;
                let mask = !(0xFFu32 << shift);
                let mut word = self.mmio_read32(aligned)?;
                word = (word & mask) | ((value as u32) << shift);
                self.mmio_write32(aligned, word)
            }
        }
    }
}

impl Bus for NovaBus {
    type Error = BusError;

    fn load8(&mut self, addr: u32) -> Result<u8, BusError> {
        if addr <= RAM_END {
            return Ok(self.ram.read8(addr)?);
        }

        if Self::in_range(addr, VRAM_BASE, VRAM_SIZE) {
            let off = addr - VRAM_BASE;
            return Ok(self.vram.read8(off)?);
        }

        if Self::in_range(addr, FONT_BASE, FONT_SIZE) {
            let off = addr - FONT_BASE;
            return Ok(self.font_ram.read8(off)?);
        }

        if addr >= MMIO_BASE && addr <= MMIO_END {
            return self.mmio_read8(addr);
        }

        Err(BusError::OutOfBounds(addr))
    }

    fn load32(&mut self, addr: u32) -> Result<u32, BusError> {
        if addr & 3 != 0 {
            return Err(BusError::Misaligned(addr));
        }

        if addr <= RAM_END {
            // ensure we don't run off the end of RAM
            if addr + 3 > RAM_END {
                return Err(BusError::OutOfBounds(addr + 3));
            }
            let b0 = self.ram.read8(addr)?;
            let b1 = self.ram.read8(addr + 1)?;
            let b2 = self.ram.read8(addr + 2)?;
            let b3 = self.ram.read8(addr + 3)?;
            return Ok(u32::from_le_bytes([b0, b1, b2, b3]));
        }

        if addr >= MMIO_BASE && addr <= MMIO_END {
            return self.mmio_read32(addr);
        }

        Err(BusError::OutOfBounds(addr))
    }

    fn store8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        if addr <= RAM_END {
            self.ram.write8(addr, value)?;
            return Ok(());
        }

        if Self::in_range(addr, VRAM_BASE, VRAM_SIZE) {
            let off = addr - VRAM_BASE;
            self.vram.write8(off, value)?;
            return Ok(());
        }

        if Self::in_range(addr, FONT_BASE, FONT_SIZE) {
            let off = addr - FONT_BASE;
            self.font_ram.write8(off, value)?;
            return Ok(());
        }

        if addr >= MMIO_BASE && addr <= MMIO_END {
            return self.mmio_write8(addr, value);
        }

        Err(BusError::OutOfBounds(addr))
    }

    fn store32(&mut self, addr: u32, value: u32) -> Result<(), BusError> {
        if addr & 3 != 0 {
            return Err(BusError::Misaligned(addr));
        }

        if addr <= RAM_END {
            self.ram.write32(addr, value)?;
            return Ok(());
        }

        if addr >= MMIO_BASE && addr <= MMIO_END {
            self.mmio_write32(addr, value)?;
        }

        Err(BusError::OutOfBounds(addr))
    }
}
