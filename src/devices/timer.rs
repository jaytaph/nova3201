pub const ENABLED: u32 = 0x1; // 0 = not running, 1 = running
pub const IRQ_ENABLED: u32 = 0x2; // 0 = no IRQ on timeout, 1 = IRQ on timeout
pub const ONE_SHOT: u32 = 0x4; // 0 = periodic, 1 = one-shot

pub struct Timer {
    /// Control register
    ctrl: u32,
    /// Current count
    counter: u32,
    /// Period to count towards
    period: u32,
    /// IRQ flag
    irq: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            ctrl: ENABLED | IRQ_ENABLED,
            counter: 0,
            period: 0,
            irq: false,
        }
    }

    pub fn period(&self) -> u32 {
        self.period
    }
    pub fn ctrl(&self) -> u32 {
        self.ctrl
    }
    pub fn count(&self) -> u32 {
        self.counter
    }
    pub fn irq(&self) -> bool {
        self.irq
    }

    pub fn reset(&mut self) {
        self.counter = 0;
    }

    pub fn set_ctrl(&mut self, ctrl: u32) {
        self.ctrl = ctrl;
    }

    pub fn set_period(&mut self, period: u32) {
        self.period = period;
        self.counter = 0;
    }

    pub fn tick(&mut self) {
        // Nothing to count towards
        if self.period == 0 {
            return;
        }

        // Timer not enabled
        if self.ctrl & ENABLED == 0 {
            return;
        }

        self.counter = self.counter.wrapping_add(1);

        if self.counter >= self.period {
            // Raise IRQ if enabled
            if self.ctrl & IRQ_ENABLED != 0 {
                self.irq = true;
            }

            // Handle one-shot vs periodic
            if self.ctrl & ONE_SHOT != 0 {
                // Just stop the timer
                self.ctrl &= !ENABLED;
            } else {
                // Automatically reset the counter
                self.counter = 0;
            }
        }
    }

    pub fn ack_irq(&mut self) {
        self.irq = false;
    }
}
