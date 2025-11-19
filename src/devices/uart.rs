pub mod pty_backend;

pub const TX_READY: u32 = 1 << 0; // 1 = Uart ready to accept TX data
pub const RX_AVAILABLE: u32 = 1 << 1; // 1 = RX data waiting
pub const IRQ_ENABLE: u32 = 1 << 7; // 0 = IRQ disabled, 1 = IRQ enabled

pub trait UartBackend: Send {
    fn read_byte(&mut self) -> Option<u8>;
    fn write_byte(&mut self, byte: u8);
}


pub struct Uart<B: UartBackend> {
    backend: B,
    status: u32,
    rx_buffer: Option<u8>,
    irq: bool,
}

impl<B: UartBackend> Uart<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            status: TX_READY,
            rx_buffer: None,
            irq: false,
        }
    }

    pub fn tick(&mut self) {
        self.poll_rx();
    }

    pub fn irq(&self) -> bool {
        self.irq
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn set_status(&mut self, val: u32) {
        // make sure some bits are read-only by masking them
        self.status = val & !(TX_READY | RX_AVAILABLE);

        // If we don't want IRQs, clear the current IRQ state
        if (self.status & IRQ_ENABLE) == 0 {
            self.irq = false;
        }
    }

    pub fn poll_rx(&mut self) {
        // Already something in the buffer
        if self.rx_buffer.is_some() {
            return;
        }

        if let Some(b) = self.backend.read_byte() {
            self.rx_buffer = Some(b);
            self.status |= RX_AVAILABLE;

            if (self.status & IRQ_ENABLE) != 0 {
                self.irq = true;
            }
        }
    }

    pub fn read_rx(&mut self) -> u8 {
        let b = self.rx_buffer.take().unwrap_or(0);

        self.status &= !RX_AVAILABLE;
        self.irq = false;

        b
    }

    pub fn write_tx(&mut self, b: u8) {
        self.backend.write_byte(b);

        self.status |= TX_READY;
    }
}
