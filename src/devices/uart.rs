use std::io::Write;

pub const TX_READY: u32 = 1 << 0; // 1 = Uart ready to accept TX data
pub const RX_AVAILABLE: u32 = 1 << 1; // 1 = RX data waiting
pub const IRQ_ENABLE: u32 = 1 << 7; // 0 = IRQ disabled, 1 = IRQ enabled

pub struct Uart {
    status: u32,
    rx_buffer: Option<u8>,
    irq: bool,
}

impl Default for Uart {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart {
    pub fn new() -> Self {
        Self {
            status: TX_READY,
            rx_buffer: None,
            irq: false,
        }
    }

    pub fn tick(&mut self) {
        // We need to check incoming reads and do IRQ hanlding here
    }

    pub fn irq(&self) -> bool {
        self.irq
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn set_status(&mut self, val: u32) {
        self.status = val;
    }

    pub fn write8(&mut self, _offset: u32, val: u8) {
        print!("{}", val as char);
        let _ = std::io::stdout().flush();
    }

    pub fn read8(&self, _offset: u32) -> u8 {
        self.rx_buffer.unwrap_or_default()
    }

    pub fn push_rx(&mut self, byte: u8) {
        self.rx_buffer = Some(byte);
        self.status |= RX_AVAILABLE;

        if (self.status & IRQ_ENABLE) != 0 {
            self.irq = true;
        }
    }
}
